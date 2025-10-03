//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{
    block_current_and_run_next, current_process, current_task, wakeup_task, TaskControlBlock,
};
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// sem id
    pub sem_id: usize,
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize, sem_id: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            sem_id,
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        let sem_id = self.sem_id;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                let task_id = task.inner_exclusive_access().res.as_ref().unwrap().tid;
                process_inner.sem_alloc[tid][sem_id] -= 1;
                process_inner.sem_need[task_id][sem_id] -= 1;
                process_inner.sem_alloc[task_id][sem_id] += 1;
                wakeup_task(task);
            }
        } else {
            process_inner.sem_alloc[tid][sem_id] -= 1;
            process_inner.sem_available[sem_id] += 1;
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
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
            let sem_id = self.sem_id;
            println!("{} {} sem", tid, sem_id);
            process_inner.sem_alloc[tid][sem_id] += 1;
            process_inner.sem_need[tid][sem_id] -= 1;
            process_inner.sem_available[sem_id] -= 1;
        }
    }
}
