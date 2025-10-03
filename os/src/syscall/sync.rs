use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec::Vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_available[id] = 1;
        for i in &mut process_inner.mutex_alloc {
            i[id] = 0;
        }
        for i in &mut process_inner.mutex_need {
            i[id] = 0;
        }
        let mutex: Option<Arc<dyn Mutex>> = if !blocking {
            Some(Arc::new(MutexSpin::new(id)))
        } else {
            Some(Arc::new(MutexBlocking::new(id)))
        };
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_available.push(1);
        for i in &mut process_inner.mutex_alloc {
            i.push(0);
        }
        for i in &mut process_inner.mutex_need {
            i.push(0);
        }
        let mutex: Option<Arc<dyn Mutex>> = if !blocking {
            Some(Arc::new(MutexSpin::new(process_inner.mutex_list.len())))
        } else {
            Some(Arc::new(MutexBlocking::new(process_inner.mutex_list.len())))
        };
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
/// 到底能不能分配呢，要打一个问号
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    process_inner.mutex_need[tid][mutex_id] += 1;
    // 开启了死锁检测
    if process_inner.detect_deadlock {
        // work 表示可用资源，finish 是一个布尔数组，表示是否可能满足第 i 个线程的愿望
        let mut work = Vec::new();
        let mut finish = Vec::new();
        for i in &process_inner.mutex_available {
            work.push(*i);
        }
        let n = process_inner.mutex_alloc.len();
        let m = process_inner.mutex_available.len();
        for _i in 0..n {
            finish.push(false);
        }

        loop {
            // flag 表示这一轮是否找到了
            let mut flag = false;
            // 外层循环枚举线程
            // 找这样一个线程，它满足 !finish 且资源都够分配走
            for i in 0..n {
                let mut tem = true;
                if !finish[i] {
                    flag = true;
                    // 内层循环枚举信号量
                    for j in 0..m {
                        if process_inner.mutex_need[i][j] > work[j] {
                            tem = false;
                            break;
                        }
                    }
                    if tem {
                        finish[i] = true;
                        for j in 0..m {
                            work[j] += process_inner.mutex_alloc[i][j];
                        }
                        break;
                    } else {
                        flag = false;
                    }
                }
            }
            if !flag {
                break;
            }
        }

        for i in 0..n {
            if !finish[i] {
                process_inner.mutex_need[tid][mutex_id] -= 1;
                return -0xDEAD;
            }
        }
    }
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.sem_available[id] = res_count as i32;
        for i in &mut process_inner.sem_alloc {
            i[id] = 0;
        }
        for i in &mut process_inner.sem_need {
            i[id] = 0;
        }
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count, id)));
        id
    } else {
        process_inner.sem_available.push(res_count as i32);
        for i in &mut process_inner.sem_alloc {
            i.push(0);
        }
        for i in &mut process_inner.sem_need {
            i.push(0);
        }
        let id = process_inner.semaphore_list.len();
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count, id))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    process_inner.sem_need[tid][sem_id] += 1;
    // 开启了死锁检测
    if process_inner.detect_deadlock {
        // work 表示可用资源，finish 是一个布尔数组，表示是否可能满足第 i 个线程的愿望
        let mut work = Vec::new();
        let mut finish = Vec::new();
        for i in &process_inner.sem_available {
            work.push(*i);
        }
        let n = process_inner.sem_alloc.len();
        let m = process_inner.sem_available.len();
        for _i in 0..n {
            finish.push(false);
        }

        loop {
            // flag 表示这一轮是否找到了
            let mut flag = false;
            // 外层循环枚举线程
            // 找这样一个线程，它满足 !finish 且资源都够分配走
            for i in 0..n {
                let mut tem = true;
                if !finish[i] {
                    flag = true;
                    // 内层循环枚举信号量
                    for j in 0..m {
                        if process_inner.sem_need[i][j] > work[j] {
                            tem = false;
                            break;
                        }
                    }
                    if tem {
                        finish[i] = true;
                        for j in 0..m {
                            work[j] += process_inner.sem_alloc[i][j];
                        }
                        break;
                    } else {
                        flag = false;
                    }
                }
            }
            if !flag {
                break;
            }
        }

        for i in 0..n {
            if !finish[i] {
                process_inner.sem_need[tid][sem_id] -= 1;
                return -0xDEAD;
            }
        }
    }
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    sem.down();
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    if _enabled >= 2 {
        return -1;
    }
    if _enabled == 1 {
        current_process().inner_exclusive_access().detect_deadlock = true;
    } else {
        current_process().inner_exclusive_access().detect_deadlock = false;
    }
    0
}
