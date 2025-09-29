## 编程作业

我的代码如下：

```rust
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
```

当 `_trace_request` 为 0 时，直接解引用裸指针读；当 `_trace_request` 为 1 时，直接解引用裸指针写；当 `_trace_request` 为 2 时，我们对每个进程控制块维护一个数组，表示系统调用号与调用次数的对应关系。这个数组在每次内核中调用 `syscall` 函数时被更新，依此实现记录每个进程调用了多少次某系统调用。



## 简答作业

### 1

在 U 态访问 S 态寄存器会报错 `[kernel] IllegalInstruction in application, kernel killed it.`。我使用 Rustsbi 版本是 0.2.2。

### 2

1. 刚进入 `__restore` 时，`sp` 的值是内核栈的栈顶地址。`__resotre` 的两个使用场景分别是刚进入内核时，需要利用 `__restore` 从内核态进入到用户态；在结束 `__alltraps` 时，进入 `trap_handler` 进行异常的具体处理，`trap_handler` 处理完之后，自动回到 `__resotre` 恢复上下文。

2. 这几行汇编代码特殊处理了 `sstatus`、`sepc`、`sscratch` 三个寄存器。`sstaus` 里保存了当前所在的特权级，回到用户态需要将这个字段从 `S` 切换到 `U`；`sepc` 里保存了返回到用户态继续执行的地址，当我们调用 `sepc` 时会将特权级切换为 `U` 然后跳转到这个地址；`sscratch` 暂存了用户栈的栈顶，当回到用户态时要换栈，让 `sp` 重新指向用户栈栈顶。
3. 因为 `x2` 是 `sp` 寄存器，我们应该存的是用户程序进入到 S 态之前的用户栈栈顶，但是现在 `sp` 已经通过进入到 `__alltraps` 的第一条指令变成了内核栈栈顶，所以不能直接像存其它寄存器一样那么存，需要后面单独特殊处理；`x4` 是 `tp` 寄存器，一般情况下不会用到，所以其实无需保存。
4. 该指令相当于交换两个 csr 的值，执行完该指令之后，`sp` 指向内核栈栈顶，`sscratch` 指向用户栈栈顶。
5. 在 `sret` 指令，因为该指令可以自动降低特权级，然后跳转到用户态中 `sepc` 对应的地址。
6. 从 U 态进入到 S 态通过 `ecall` 发生。



## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我没有与他人进行交流。
2. 此外，没有参考其它资料。
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。
5. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。