//! # 有栈协程与上下文切换（riscv64）
//!
//! 在这个练习中，你将用内联汇编实现最小化的上下文切换——这是 OS 线程调度的核心机制。
//! 本 crate **仅限 riscv64**；在 riscv64 Linux 上跑 `cargo test`，或在 x86 上用仓库的正常流程
//! （`./check.sh` / `oscamp`，通过 QEMU 模拟）。
//!
//! ## 关键概念
//! - **被调用者保存寄存器（callee-saved）**：切换时保存并恢复它们，被切走的任务之后才能正确续跑。
//! - **栈指针 `sp`** 和 **返回地址 `ra`**：在新上下文里恢复它们；第一次切换到某个任务时，`ret` 会跳到 `ra`（即入口点）。
//! - 内联汇编：`core::arch::asm!`
//!
//! ## riscv64 ABI（本练习用到的部分）
//! - 被调用者保存：`sp`、`ra`、`s0`–`s11`。`ret` 指令即 `jalr zero, 0(ra)`。
//! - 第一个和第二个参数：`a0`（旧上下文）、`a1`（新上下文）。

#![cfg(target_arch = "riscv64")]

/// 一个任务的保存寄存器状态（riscv64）。内存布局必须与下面汇编使用的偏移一致：
/// `sp` 在 0，`ra` 在 8，`s0`–`s11` 依次在 16、24、… 104。
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TaskContext {
    pub sp: u64,
    pub ra: u64,
    pub s0: u64,
    pub s1: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
}

impl TaskContext {
    pub const fn empty() -> Self {
        Self {
            sp: 0,
            ra: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }

    /// 初始化此上下文：切换过去时，执行从 `entry` 开始。
    ///
    /// - 设 `ra = entry`，让新上下文的第一个 `ret` 跳到 `entry`。
    /// - 设 `sp = stack_top`，并保持 16 字节对齐（RISC-V ABI 要求函数入口处栈 16 字节对齐）。
    /// - `s0`–`s11` 留零；切换时会被装载。
    pub fn init(&mut self, stack_top: usize, entry: usize) {
        // todo!("set ra = entry, sp = stack_top (16-byte aligned)");
        self.ra = entry as u64;
        self.sp = stack_top as u64;
    }
}

/// 从 `old` 上下文切换到 `new` 上下文：把当前的被调用者保存寄存器存入 `old`，
/// 从 `new` 装载，然后 `ret`（跳向 `new.ra`）。
///
/// 汇编层面：把 `sp`、`ra`、`s0`–`s11` 存到 `[a0]`（old），从 `[a1]`（new）装载，
/// 将 `a0`/`a1` 清零以免把指针泄漏进新上下文，最后 `ret`。
///
/// 必须标注 `#[unsafe(naked)]`，以阻止编译器生成序言/尾声。
#[unsafe(naked)]
pub unsafe extern "C" fn switch_context(old: &mut TaskContext, new: &TaskContext) {
    core::arch::naked_asm!(
        // ── 存档：CPU 现场 → 旧上下文（a0）──
        "sd sp, 0(a0)",      // 把 sp 存到第 0 格
        "sd ra, 8(a0)",      // 把 ra 存到第 1 格
        "sd s0, 16(a0)",     // 把 s0 存到第 2 格
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
        "sd s11, 104(a0)",   // 14 个格子存完
        // ── 读档：新上下文（a1）→ CPU ──
        "ld sp, 0(a1)",      // 从第 0 格读出装进 sp（从此站上新栈）
        "ld ra, 8(a1)",
        "ld s0, 16(a1)",
        "ld s1, 24(a1)",
        "ld s2, 32(a1)",m
        "ld s3, 40(a1)",
        "ld s4, 48(a1)",
        "ld s5, 56(a1)",
        "ld s6, 64(a1)",
        "ld s7, 72(a1)",
        "ld s8, 80(a1)",
        "ld s9, 88(a1)",
        "ld s10, 96(a1)",
        "ld s11, 104(a1)",   // 14 个格子装完
        // ── 卫生：清指针，不泄漏进新上下文 ──
        "li a0, 0",
        "li a1, 0",
        "ret",               // 跳向刚装好的 ra
    );
}

const STACK_SIZE: usize = 1024 * 64;

/// 为协程分配一个栈。返回 `(buffer, stack_top)`，其中 `stack_top` 是高地址
/// （栈向低地址生长）。buffer 必须在使用它的上下文存活期间一直活着。
pub fn alloc_stack() -> (Vec<u8>, usize) {
    let buf = vec![0u8; STACK_SIZE];
    let top = buf.as_ptr() as usize + STACK_SIZE;   // 高地址端 = 起点 + 大小
    (buf, top)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    extern "C" fn task_entry() {
        COUNTER.store(42, Ordering::SeqCst);
        loop {
            std::hint::spin_loop();
        }
    }

    #[test]
    fn test_alloc_stack() {
        let (buf, top) = alloc_stack();
        assert_eq!(top, buf.as_ptr() as usize + STACK_SIZE);
        assert!(top % 16 == 0);
    }

    #[test]
    fn test_context_init() {
        let (buf, top) = alloc_stack();
        let _ = buf;
        let mut ctx = TaskContext::empty();
        let entry = task_entry as *const () as usize;
        ctx.init(top, entry);
        assert_eq!(ctx.ra, entry as u64);
        assert!(ctx.sp != 0);
    }

    #[test]
    fn test_switch_to_task() {
        COUNTER.store(0, Ordering::SeqCst);

        static mut MAIN_CTX_PTR: *mut TaskContext = std::ptr::null_mut();
        static mut TASK_CTX_PTR: *mut TaskContext = std::ptr::null_mut();

        extern "C" fn cooperative_task() {
            COUNTER.store(99, Ordering::SeqCst);
            unsafe {
                switch_context(&mut *TASK_CTX_PTR, &*MAIN_CTX_PTR);
            }
        }

        let (_stack_buf, stack_top) = alloc_stack();
        let mut main_ctx = TaskContext::empty();
        let mut task_ctx = TaskContext::empty();
        task_ctx.init(stack_top, cooperative_task as *const () as usize);

        unsafe {
            MAIN_CTX_PTR = &mut main_ctx;
            TASK_CTX_PTR = &mut task_ctx;
            switch_context(&mut main_ctx, &task_ctx);
        }

        assert_eq!(COUNTER.load(Ordering::SeqCst), 99);
    }
}
