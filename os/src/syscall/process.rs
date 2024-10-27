//! Process management syscalls
use crate::{
    config::{MAX_APP_NUM, MAX_SYSCALL_NUM}, loader::get_num_app, task::{exit_current_and_run_next, suspend_current_and_run_next, TaskStatus}, timer::get_time_us
};

use lazy_static::*;
use crate::sync::UPSafeCell;
use crate::timer::get_time_ms;
use crate::task::get_current_task_id;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

#[allow(dead_code)]
pub struct TaskInfoList {
    pub task_num: usize,
    pub task_info: UPSafeCell<[TaskInfo; MAX_APP_NUM]>,
}

#[allow(dead_code)]
impl TaskInfoList {
    pub fn update_syscall_times(&self, syscall_id: usize,task_id :usize) {
        let mut task_info = self.task_info.exclusive_access();
        task_info[task_id].syscall_times[syscall_id] += 1;
        drop(task_info);
    }

    pub fn mark_first_task_running(&self, task_id: usize) {
        let mut task_info = self.task_info.exclusive_access();
        //记录第一次运行时间
        if task_info[task_id].time == 0 {
            task_info[task_id].time = get_time_ms();
            drop(task_info);            
        }
        else{
            //直接退出
            drop(task_info);
        }
    }

}

//！ Global variable: TASK_INFO
lazy_static! {
    /// Global variable: TASK_INFO
    pub static ref TASK_INFO: TaskInfoList = {
        let task_num = get_num_app();
        let task_infos = [TaskInfo {
            status: TaskStatus::Running,
            syscall_times: [0; MAX_SYSCALL_NUM],
            time: 0,
        }; MAX_APP_NUM];
        TaskInfoList {
            task_num,
            task_info: unsafe {
                UPSafeCell::new(task_infos)
            },
        }
    };
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let task_info = TASK_INFO.task_info.exclusive_access();
    let current = get_current_task_id(); //获取当前任务id
    let time = get_time_ms() - task_info[current].time; //获取运行时间
    let status = task_info[current].status; //获取状态
    let syscall_times = task_info[current].syscall_times; //获取系统调用次数
    drop(task_info);
    unsafe {
        *_ti = TaskInfo {
            status,
            syscall_times,
            time,
        };
    }
    0
}

