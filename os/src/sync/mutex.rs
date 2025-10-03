//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_process, TaskControlBlock};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
    /// get mutex id
    fn mutex_id(&self) -> usize;
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    mutex_id: usize,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new(mutex_id: usize) -> Self {
        Self {
            mutex_id,
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                let tid = current_task()
                    .unwrap()
                    .inner_exclusive_access()
                    .res
                    .as_ref()
                    .unwrap()
                    .tid;
                let process = current_process();
                let mut process_inner = process.inner_exclusive_access();
                let mutex_id = self.mutex_id;
                process_inner.mutex_alloc[tid][mutex_id] += 1;
                process_inner.mutex_need[tid][mutex_id] -= 1;
                process_inner.mutex_available[mutex_id] -= 1;
                return;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        let mutex_id = self.mutex_id;
        process_inner.mutex_alloc[tid][mutex_id] -= 1;
        process_inner.mutex_available[mutex_id] += 1;
        *locked = false;
    }

    fn mutex_id(&self) -> usize {
        self.mutex_id
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    mutex_id: usize,
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new(mutex_id: usize) -> Self {
        trace!("kernel: MutexBlocking::new");
        println!("kernel: MutexBlocking::new {}", mutex_id);
        Self {
            mutex_id,
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            let tid = current_task()
                .unwrap()
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .tid;
            let process = current_process();
            let mut process_inner = process.inner_exclusive_access();
            let mutex_id = self.mutex_id;
            println!("{} {} mutexblocking", tid, mutex_id);
            process_inner.mutex_alloc[tid][mutex_id] += 1;
            process_inner.mutex_need[tid][mutex_id] -= 1;
            process_inner.mutex_available[mutex_id] -= 1;
            mutex_inner.locked = true;
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        let mutex_id = self.mutex_id;
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            let task_id = waking_task
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .tid;
            process_inner.mutex_alloc[tid][mutex_id] -= 1;
            process_inner.mutex_need[task_id][mutex_id] -= 1;
            process_inner.mutex_alloc[task_id][mutex_id] += 1;
            wakeup_task(waking_task);
        } else {
            process_inner.mutex_alloc[tid][mutex_id] -= 1;
            process_inner.mutex_available[mutex_id] += 1;
            mutex_inner.locked = false;
        }
    }

    fn mutex_id(&self) -> usize {
        self.mutex_id
    }
}
