//! # 绿色线程调度器（riscv64）
//!
//! 在这个练习中，你将在上下文切换之上搭建一个简单的协作式（绿色）线程调度器。
//! 本 crate **仅限 riscv64**；用仓库的正常流程（`./check.sh` / `oscamp`）或在 riscv64 上原生运行。
//!
//! ## 关键概念
//! - 协作式调度 vs 抢占式调度
//! - 线程状态：`Ready`（就绪）、`Running`（运行中）、`Finished`（已结束）
//! - `yield_now()`：当前线程主动让出 CPU
//! - 调度循环：挑出下一个就绪线程并切换过去
//!
//! ## 设计
//! 每个绿色线程有自己的栈和 `TaskContext`。线程调用 `yield_now()` 主动让出。
//! 调度器在就绪线程之间轮转（round-robin）。用户入口被 `thread_wrapper` 包裹：
//! wrapper 先调用入口函数，然后把线程标记为 `Finished` 并切回。

#![cfg(target_arch = "riscv64")]

use core::arch::naked_asm;

/// 每线程栈大小。稍微调大以避免在 QEMU / 测试环境下栈溢出。
const STACK_SIZE: usize = 1024 * 128;

/// 任务上下文（riscv64）；布局必须与 `01_stack_coroutine::TaskContext` 及下面的汇编一致。
#[repr(C)]
#[derive(Debug, Default, Clone)]
pub struct TaskContext {
    sp: u64,
    ra: u64,
    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
    s10: u64,
    s11: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreadState {
    Ready,
    Running,
    Finished,
}

struct GreenThread {
    ctx: TaskContext,
    state: ThreadState,
    _stack: Option<Vec<u8>>,
    /// 用户入口；线程第一次被调度时取走（take），交给 `thread_wrapper` 调用。
    entry: Option<extern "C" fn()>,
}

/// 由调度器在切换到新线程之前设置；`thread_wrapper` 读取它并只调用一次。
static mut CURRENT_THREAD_ENTRY: Option<extern "C" fn()> = None;

/// 作为每个绿色线程初始 `ra` 运行的包裹函数：调用用户入口（从 `CURRENT_THREAD_ENTRY` 取），
/// 然后标记为 Finished 并切回。
extern "C" fn thread_wrapper() {
    let entry = unsafe { core::ptr::read(&raw const CURRENT_THREAD_ENTRY) };
    if let Some(f) = entry {
        unsafe { CURRENT_THREAD_ENTRY = None };
        f();
    }
    thread_finished();
}

/// 把当前被调用者保存寄存器存入 `old`，从 `new` 装载，然后 `ret` 跳向 `new.ra`。
/// 在 `ret` 前将 `a0`/`a1` 清零，避免把指针泄漏进新上下文。
///
/// 必须标注 `#[unsafe(naked)]`，以阻止编译器生成序言/尾声。
#[unsafe(naked)]
unsafe extern "C" fn switch_context(_old: &mut TaskContext, _new: &TaskContext) {
    naked_asm!(
        "sd sp, 0(a0)",
        "sd ra, 8(a0)",
        "sd s0, 16(a0)",
        "sd s1, 24(a0)",
        "sd s2, 32(a0)",
        "sd s3, 40(a0)",
        "sd s4, 48(a0)",
        "sd s5, 56(a0)",
        "sd s6, 64(a0)",
        "sd s7, 72(a0)",
        "sd s8, 80(a0)",
        "sd s9, 88(a0)",
        "sd s10, 96(a0)",
        "sd s11, 104(a0)",
        "ld sp, 0(a1)",
        "ld ra, 8(a1)",
        "ld s0, 16(a1)",
        "ld s1, 24(a1)",
        "ld s2, 32(a1)",
        "ld s3, 40(a1)",
        "ld s4, 48(a1)",
        "ld s5, 56(a1)",
        "ld s6, 64(a1)",
        "ld s7, 72(a1)",
        "ld s8, 80(a1)",
        "ld s9, 88(a1)",
        "ld s10, 96(a1)",
        "ld s11, 104(a1)",
        "li a0, 0",
        "li a1, 0",
        "ret",
    );
}

pub struct Scheduler {
    threads: Vec<GreenThread>,
    current: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        let main_thread = GreenThread {
            ctx: TaskContext::default(),
            state: ThreadState::Running,
            _stack: None,
            entry: None,
        };

        Self {
            threads: vec![main_thread],
            current: 0,
        }
    }

    /// 注册一个新的绿色线程，首次被调度时将运行 `entry`。
    ///
    /// 1. 分配 `STACK_SIZE` 字节的栈；计算 `stack_top`（高地址）。
    /// 2. 布置上下文：`ra = thread_wrapper`，让第一次切换跳到 wrapper；
    ///    `sp` 必须 16 字节对齐（例如 `(stack_top - 16) & !15` 留出余量）。
    /// 3. 压入一个 `GreenThread`：带上此上下文、状态 `Ready`，并把 `entry` 存起来供 wrapper 调用。
    pub fn spawn(&mut self, entry: extern "C" fn()) {
        // ① 在堆上分配一块 STACK_SIZE 大小的内存作为该线程的私有栈（全零初始化）
        let buf = vec![0u8; STACK_SIZE];
        // ② 栈顶 = 缓冲区起始地址 + 总大小（高地址端，栈向低地址生长）
        let stack_top = buf.as_ptr() as usize + STACK_SIZE;
        // ③ 布置出生姿势：全零上下文起步
        let mut ctx = TaskContext::default();
        //    ra 指向 thread_wrapper（不是 entry！）——第一次被切过去时 ret 落进 wrapper，
        //    它负责取 entry、调用、跑完后收尸（标 Finished 并切走）
        ctx.ra = thread_wrapper as *const () as usize as u64;
        //    sp = 栈顶往下留 16 字节余量（防第一次压栈越界），再 & !15 做 16 字节对齐（ABI 要求）
        ctx.sp = ((stack_top - 16) & !15) as u64;

        // ④ 入队：排队等叫号（Ready）；栈的所有权存进线程结构体保活；entry 存着等 wrapper 出生时取用
        self.threads.push(GreenThread {
            ctx,
            state: ThreadState::Ready,
            _stack: Some(buf),
            entry: Some(entry),
        });
    }
    

    /// 运行调度器，直到除主线程外的所有线程都 `Finished`。
    ///
    /// 1. 把全局 `SCHEDULER` 指针设为 `self`，让 `yield_now` 和 `thread_finished` 能回调。
    /// 2. 循环：若 `threads[1..]` 全部 `Finished` 则退出；否则调用 `schedule_next()`
    ///    （它可能切走，之后某刻会切回来）。
    /// 3. 结束时清空 `SCHEDULER`。
    pub fn run(&mut self) {
        // ① 上岗登记：yield_now / thread_finished 都在别的线程的栈上跑，
        //    它们靠这个全局指针找回调度器
        unsafe { SCHEDULER = self; }

        loop {
            // ② 退出条件：threads[1..] 全部 Finished（跳过 index 0 = 主线程自己，
            //    主线程永远不该计入"完成"）
            let all_done = self.threads[1..]
                .iter()
                .all(|t| t.state == ThreadState::Finished);
            if all_done {
                break;
            }
            // 切出去干活；某次轮转切回主线程时，ret 把执行流送回这行之后——loop 继续
            self.schedule_next();
        }

        // ③ 下班卸任：清掉全局指针，防止之后误用
        unsafe { SCHEDULER = std::ptr::null_mut(); }
    }

    /// 找出下一个就绪线程（从 `current + 1` 开始轮转），把当前线程标记为 `Ready`（若不是 `Finished`），
    /// 把下一个标记为 `Running`，若下一个线程有 entry 则设置 `CURRENT_THREAD_ENTRY`，然后切换过去。
    fn schedule_next(&mut self) {
        // ① 当前线程记状态：yield 离场的退回 Ready（还能再上）；
        //    跑完 return 的已被 thread_finished 标成 Finished，不能覆盖（否则诈尸）
        if self.threads[self.current].state != ThreadState::Finished {
            self.threads[self.current].state = ThreadState::Ready;
        }

        // ② 轮转（round-robin）：从 current+1 开始绕圈找下一个 Ready
        //    （刚让出的人排到队尾；% len 扫过末尾回到 0，主线程也能被切回）
        let len = self.threads.len();
        let mut next = None;
        for i in 1..=len {
            let idx = (self.current + i) % len;
            if self.threads[idx].state == ThreadState::Ready {
                next = Some(idx);
                break;
            }
        }
        let next = next.expect("no ready thread");

        // ③ 放出生卡：还没出生的线程（entry 仍是 Some），把它要跑的函数
        //    放上全局槽——wrapper 出生后第一件事从这里取；已出生的 entry 是 None，跳过
        if let Some(e) = self.threads[next].entry {
            unsafe { CURRENT_THREAD_ENTRY = Some(e); }
        }

        // 正式叫号：下一个线程进入 Running
        self.threads[next].state = ThreadState::Running;

        // ④ 切换！先记账再切——切回来时 current 已经是新的
        let old = self.current;
        self.current = next;
        // 借用检查解法：先各自转成裸指针（借用立刻结束），再在 unsafe 里解引用。
        // 安全性由 old != next（两个不同线程的槽位）担保
        let old_ctx = self.threads[old].ctx.as_mut_ptr();
        let new_ctx = self.threads[next].ctx.as_ptr();
        unsafe {
            switch_context(&mut *old_ctx, &*new_ctx);
        }
    }
}

impl TaskContext {
    fn as_mut_ptr(&mut self) -> *mut TaskContext {
        self as *mut TaskContext
    }
    fn as_ptr(&self) -> *const TaskContext {
        self as *const TaskContext
    }
}

static mut SCHEDULER: *mut Scheduler = std::ptr::null_mut();

/// 当前线程主动让出；调度器会挑下一个就绪线程。
pub fn yield_now() {
    unsafe {
        if !SCHEDULER.is_null() {
            (*SCHEDULER).schedule_next();
        }
    }
}

/// 把当前线程标记为 `Finished` 并切换到下一个（由 `thread_wrapper` 在用户入口返回后调用）。
fn thread_finished() {
    unsafe {
        if !SCHEDULER.is_null() {
            let sched = &mut *SCHEDULER;
            sched.threads[sched.current].state = ThreadState::Finished;
            sched.schedule_next();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    /// 测试必须串行运行：调度器使用全局状态（SCHEDULER、CURRENT_THREAD_ENTRY）。
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    static EXEC_ORDER: AtomicU32 = AtomicU32::new(0);

    extern "C" fn task_a() {
        EXEC_ORDER.fetch_add(1, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(10, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(100, Ordering::SeqCst);
    }

    extern "C" fn task_b() {
        EXEC_ORDER.fetch_add(1, Ordering::SeqCst);
        yield_now();
        EXEC_ORDER.fetch_add(10, Ordering::SeqCst);
    }

    #[test]
    fn test_scheduler_runs_all() {
        let _guard = TEST_LOCK.lock().unwrap();
        EXEC_ORDER.store(0, Ordering::SeqCst);

        let mut sched = Scheduler::new();
        sched.spawn(task_a);
        sched.spawn(task_b);
        sched.run();

        let got = EXEC_ORDER.load(Ordering::SeqCst);
        if got != 122 {
            panic!(
                "EXEC_ORDER: 期望 122，实际 {}（用 --nocapture 运行可看 stderr）",
                got
            );
        }
    }

    static SIMPLE_FLAG: AtomicU32 = AtomicU32::new(0);

    extern "C" fn simple_task() {
        SIMPLE_FLAG.store(42, Ordering::SeqCst);
    }

    #[test]
    fn test_single_thread() {
        let _guard = TEST_LOCK.lock().unwrap();
        SIMPLE_FLAG.store(0, Ordering::SeqCst);

        let mut sched = Scheduler::new();
        sched.spawn(simple_task);
        sched.run();

        assert_eq!(SIMPLE_FLAG.load(Ordering::SeqCst), 42);
    }
}
