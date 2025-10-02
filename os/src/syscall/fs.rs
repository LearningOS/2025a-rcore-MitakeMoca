//! File and filesystem-related syscalls
use core::mem;

use crate::fs::{open_file, InodeType, OSInode, OpenFlags, Stat, StatMode, ROOT_INODE};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
/// 千万别忘了，传进来的这个 _st 是一个用户态的虚拟地址，所以要先地址转换
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    let st: &mut &'static mut [u8] = &mut translated_byte_buffer(
        current_user_token(),
        _st as *const u8,
        mem::size_of::<Stat>(),
    )[0];
    let raw_ptr = st.as_mut_ptr();
    let st = raw_ptr as *mut Stat;
    unsafe {
        (*st).dev = 0;
        let ff = current_task().unwrap().inner_exclusive_access().fd_table[_fd]
            .clone()
            .unwrap();
        let ff = ff.as_any();
        if let Some(inode) = ff.downcast_ref::<OSInode>() {
            let inner = inode.inner.exclusive_access();
            (*st).ino = inner.inode.get_ino();
            println!("zhouzhou");
            match inode.inode_type {
                InodeType::File => (*st).mode = StatMode::FILE,
                InodeType::Dir => (*st).mode = StatMode::DIR,
                _ => (*st).mode = StatMode::NULL,
            }
            (*st).nlink = ROOT_INODE.count_link((*st).ino as u32);
        }
    }
    0
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    let token = current_user_token();
    let old_name = translated_str(token, _old_name);
    let new_name = translated_str(token, _new_name);
    if old_name == new_name {
        return -1;
    }
    let inode_id = ROOT_INODE
        .read_disk_inode(|disk_inode| ROOT_INODE.find_inode_id(&old_name, disk_inode))
        .unwrap();
    ROOT_INODE.link_at(inode_id, &new_name);
    0
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    let token = current_user_token();
    let name = translated_str(token, _name);
    ROOT_INODE.unlink(&name)
}
