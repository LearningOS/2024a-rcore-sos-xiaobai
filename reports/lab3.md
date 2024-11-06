# 实验实现的功能
> - 本次实验实现了以下功能：  
fn sys_spawn(path: *const u8) -> isize  
功能：新建子进程，使其执行目标程序。说明：成功返回子进程id，否则返回 -1。  
fn sys_set_priority(prio: isize) -> isize;  
设置当前进程优先级为 prio,参数：prio 进程优先级，要求 prio >= 2,返回值：如果输入合法则返回 prio，否则返回 -1  


# 简答作业
## 实际情况是轮到 p1 执行吗？为什么？  
由于 stride 值是无符号整形，存在溢出问题。当 p2 执行一个时间片后，其 stride 值会变为 260，但由于使用 8 位存储，实际值为 260 % 256 = 4。此时 p1.stride = 255，p2.stride = 4，因此下一次调度时，p2 会再次被选择执行。 
## 解决方案  
```rust
//特殊的比法
impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let half_max_stride = max_stride / 2;
        let diff = self.0.wrapping_sub(other.0);
        if diff < half_max_stride {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}
//实现相等这tarit
impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}    
```
# 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    > 交流的对象：chatgpt-4o / github copilot  
    交流的内容：rust的语法，汇编指令，寄存器的作用，异常处理，上下文切换等知识。

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    >参考书籍：[Detail Book rCore-Tutorial-Book-v3](https://rcore-os.cn/rCore-Tutorial-Book-v3/chapter0/1what-is-os.html)

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。