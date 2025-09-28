//! Process management syscalls
use core::mem;

use crate::{
    config::PAGE_SIZE,
    mm::{translated_byte_buffer, MapPermission, PTEFlags, PageTable, VirtAddr},
    syscall::SYSCALL_GET_TIME,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TASK_MANAGER,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
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
    let us = get_time_us();
    // 为什么这个系统调用需要修改呢
    // 因为 _ts 是用户态地址空间的一个虚拟地址，你现在是在内核态的地址空间下了
    let ts = &mut translated_byte_buffer(
        current_user_token(),
        _ts as *const u8,
        mem::size_of::<TimeVal>(),
    )[0];
    let raw_ptr = ts.as_mut_ptr();
    let ts = raw_ptr as *mut TimeVal;
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    // 这要改的一个问题是 _id 对应的内存不一定可读或者可写
    // 而且这个也变成用户地址空间的虚拟地址了
    match _trace_request {
        0 => {
            // 拿到当前用户进程的地址空间
            let page_table = PageTable::from_token(current_user_token());
            let addr = VirtAddr::from(_id);
            match page_table.find_pte(addr.floor()) {
                None => return -1,
                Some(x) => {
                    if (x.bits & (PTEFlags::U.bits() as usize | PTEFlags::R.bits() as usize))
                        != (PTEFlags::U.bits() as usize | PTEFlags::R.bits() as usize)
                    {
                        return -1;
                    }
                    let ts =
                        &mut translated_byte_buffer(current_user_token(), _id as *const u8, 1)[0];
                    let raw_ptr = ts.as_mut_ptr();
                    let val = unsafe { *(raw_ptr as *const isize) };
                    return val;
                }
            }
        }
        1 => {
            // 拿到当前用户进程的地址空间
            let page_table = PageTable::from_token(current_user_token());
            let addr = VirtAddr::from(_id);
            match page_table.find_pte(addr.floor()) {
                None => return -1,
                Some(x) => {
                    if (x.bits & (PTEFlags::U.bits() as usize | PTEFlags::W.bits() as usize))
                        != (PTEFlags::U.bits() as usize | PTEFlags::W.bits() as usize)
                    {
                        return -1;
                    }
                    let ts =
                        &mut translated_byte_buffer(current_user_token(), _id as *const u8, 1)[0];
                    let raw_ptr = ts.as_mut_ptr();
                    unsafe { *(raw_ptr as *mut usize) = _data };
                    return 0;
                }
            }
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

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    if ((_port & !0x7) != 0) || ((_port & 0x7) == 0) {
        return -1;
    }
    let end = _start + _len - 1;
    let start_page = _start / PAGE_SIZE;
    let end_page = end / PAGE_SIZE;
    let page_table = PageTable::from_token(current_user_token());
    for i in start_page..end_page + 1 {
        if let Some(_x) = page_table.find_pte(i.into()) {
            return -1;
        };
    }
    let num = TASK_MANAGER.get_inner().get_current_task();
    let perm = MapPermission::from_bits((_port << 1 | (1 << 4)) as u8).unwrap();

    TASK_MANAGER.get_inner().tasks[num]
        .memory_set
        .insert_framed_area(_start.into(), end.into(), perm);
    println!("zhouzhou");
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    let end = _start + _len - 1;
    let start_page = _start / PAGE_SIZE;
    let end_page = end / PAGE_SIZE;
    let mut page_table = PageTable::from_token(current_user_token());
    for i in start_page..end_page + 1 {
        if let None = page_table.find_pte(i.into()) {
            return -1;
        };
    }
    for i in start_page..end_page + 1 {
        page_table.unmap(i.into());
    }
    0
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
