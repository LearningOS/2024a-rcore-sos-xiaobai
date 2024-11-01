//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str,translated_byte_buffer,VirtAddr,VirtPageNum},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

impl TimeVal {
    /// 将TimeVal转换为字节数组
    pub fn to_bytes(&self) -> [u8; 2 * core::mem::size_of::<usize>()] {
        let mut bytes = [0u8; 2 * core::mem::size_of::<usize>()];
        // 将 sec 和 usec 转换为字节数组
        let sec_bytes = self.usize_to_bytes(self.sec);
        let usec_bytes = self.usize_to_bytes(self.usec);
        
        let usize_size = core::mem::size_of::<usize>();
        // 将 sec 和 usec 的字节值填充到 bytes 数组中
        for i in 0..usize_size {
            bytes[i] = sec_bytes[i];
            bytes[i + usize_size] = usec_bytes[i];
        }
        
        bytes
    }

    /// 将usize值转换为字节数组
    fn usize_to_bytes(&self, val: usize) -> [u8; core::mem::size_of::<usize>()] {
        let mut arr = [0u8; core::mem::size_of::<usize>()];
        // 将 usize 值转换为字节数组
        for i in 0..core::mem::size_of::<usize>() {
            arr[i] = ((val >> (i * 8)) & 0xFF) as u8;
        }
        arr
    }
    
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
}

impl TaskInfo {
    /// 创建一个新的TaskInfo实例
    #[allow(dead_code)]
    pub fn new(status: TaskStatus, syscall_times: [u32; MAX_SYSCALL_NUM], time: usize) -> Self {
        TaskInfo {
            status,
            syscall_times,
            time,
        }
    }
    /// 将TaskInfo转换为字节数组
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> [u8; core::mem::size_of::<TaskInfo>()] {
        let ptr = self as *const _ as *const u8;
        let mut bytes = [0u8; core::mem::size_of::<TaskInfo>()];
        unsafe {
            for i in 0..core::mem::size_of::<TaskInfo>() {
                bytes[i] = *ptr.add(i);
            }
        }
        bytes
    }

    /// 从字节数组中创建TaskInfo
    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8; core::mem::size_of::<TaskInfo>()]) -> TaskInfo {
        unsafe {
            core::mem::transmute::<[u8; core::mem::size_of::<TaskInfo>()], TaskInfo>(*bytes)
        }
    }
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // 获取当前时间（微秒）
    let us = crate::timer::get_time_us();
    // 创建 TimeVal 结构体
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    // 将 TimeVal 结构体转换为字节数组
    let serialized = time_val.to_bytes();
    // 将用户空间的指针转换为内核空间的字节缓冲区
    let mut buffers = translated_byte_buffer(current_user_token(), _ts as *const u8, serialized.len());
    // 将 serialized 的内容复制到用户空间
    // 手动复制 serialized 的内容到用户空间
    for i in 0..buffers.len() {
        let buffer = &mut buffers[i];
        for j in 0..buffer.len() {
            buffer[j] = serialized[i * buffer.len() + j];
        }
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    // 获取当前任务的task_manager
    let current_task = current_task().unwrap();
    let inner = current_task.inner_exclusive_access();
    let task_info = inner.get_taskinfo();
    drop(inner);
    // 将 TaskInfo 结构体转换为字节数组
    let serialized = task_info.to_bytes();
    // 将用户空间的指针转换为内核空间的字节缓冲区
    let mut buffers = translated_byte_buffer(current_user_token(), _ti as *const u8, serialized.len());
    // 将 serialized 的内容复制到用户空间
    // 手动复制 serialized 的内容到用户空间
    for i in 0..buffers.len() {
        let buffer = &mut buffers[i];
        for j in 0..buffer.len() {
            buffer[j] = serialized[i * buffer.len() + j];
        }
    }
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    // 将起始地址转换为虚拟地址
    let start_vaddr: VirtAddr = _start.into();
    // 检查起始地址是否已对齐
    if !start_vaddr.aligned() {
        debug!("映射失败：起始地址未对齐");
        return -1;
    }
    // 检查port的有效性
    if _port & !0x7 != 0 || _port & 0x7 == 0 {
        return -1;
    }
    // 如果长度为0，则直接返回
    if _len == 0 {
        return 0;
    }
    // 计算结束地址
    let end_vaddr: VirtAddr = (_start + _len).into();
    let start_vpn: VirtPageNum = start_vaddr.into();
    let end_vpn: VirtPageNum = (end_vaddr).ceil();
    //调用task封装好的全局map函数
    crate::task::mmap(start_vpn, end_vpn, _port)
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    // 将起始地址转换为虚拟地址
    let start_vaddr: VirtAddr = _start.into();
    // 检查起始地址是否已对齐
    if !start_vaddr.aligned() {
        debug!("映射失败：起始地址未对齐");
        return -1;
    }
    // 如果长度为0，则直接返回
    if _len == 0 {
        return 0;
    }
    // 计算结束地址
    let end_vaddr: VirtAddr = (_start + _len).into();
    let start_vpn: VirtPageNum = start_vaddr.into();  // 向下取整
    let end_vpn: VirtPageNum = (end_vaddr).ceil(); // 向上取整
    //调用task封装好的全局unmap函数
    crate::task::munmap(start_vpn, end_vpn)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let t = current_task().unwrap();
    let mut inner = t.inner_exclusive_access();
    let token = inner.get_user_token();
    let path = translated_str(token, path);

    //println!("kernel:pid[{}]", current_task().unwrap().pid.0);

    if let Some(data) = get_app_data_by_name(path.as_str()) {
        // 创建新的任务控制块
        let child_tcb = Arc::new(crate::task::TaskControlBlock::new(data));
        {
            let mut child_inner = child_tcb.inner_exclusive_access();
            // 设置子进程的父进程
            child_inner.parent = Some(Arc::downgrade(&t));
        }
        // 将子进程加入父进程的子进程列表
        inner.children.push(child_tcb.clone());
        // 将子进程添加到任务调度器
        add_task(child_tcb.clone());
        //println!("children_process: {},pid_num: {}", path,child_tcb.pid.0);
        return child_tcb.pid.0 as isize;
    } else {
        return -1;
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    crate::task::set_current_task_priority(_prio)
}
