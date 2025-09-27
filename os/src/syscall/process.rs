//! Process management syscalls

use crate::{
    syscall::SYSCALL_GET_TIME,
    task::{exit_current_and_run_next, suspend_current_and_run_next, TASK_MANAGER},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
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

pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    match _trace_request {
        0 => {
            let val = unsafe { *(_id as *const isize) };
            return val;
        }
        1 => {
            unsafe { *(_id as *mut usize) = _data };
            return 0;
        }
        2 => {
            let num = TASK_MANAGER.get_inner().get_current_task();
            let inner = TASK_MANAGER.get_inner();
            if _id == SYSCALL_GET_TIME {
                println!("{} {}", _id, inner.tasks[num].syscalls[_id] as isize);
            }
            return inner.tasks[num].syscalls[_id] as isize;
        }
        _ => {
            return -1;
        }
    }
}
