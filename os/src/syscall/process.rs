//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM, 
    mm::{translated_byte_buffer,VirtPageNum,VirtAddr},
    task::{change_program_brk, current_user_token, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus, TASK_MANAGER,mmap,munmap},
    timer::get_time_ms,
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
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
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
    let task_manager = TASK_MANAGER.inner.exclusive_access();
    // 获取当前任务的任务号
    let current_task = task_manager.current_task;
    // 获取当前任务的TaskInfo
    let status = task_manager.tasks[current_task].task_status;
    let syscall_times = task_manager.tasks[current_task].task_syscall_times;
    let time = get_time_ms() - task_manager.tasks[current_task].task_time;
    // 创建 TaskInfo 结构体
    let task_info = TaskInfo {
        status,
        syscall_times,
        time,
    };
    drop(task_manager); // 释放task_manager
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

// YOUR JOB: Implement mmap.
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
    mmap(start_vpn, end_vpn, _port)
}

// YOUR JOB: Implement munmap.
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
    munmap(start_vpn, end_vpn)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
