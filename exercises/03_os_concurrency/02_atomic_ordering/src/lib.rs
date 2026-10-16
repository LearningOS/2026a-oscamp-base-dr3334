//! # 内存顺序与同步
//!
//! 在这个练习中，你将使用正确的内存顺序实现线程同步原语。
//!
//! ## 关键概念
//! - `Ordering::Relaxed`：无同步保证
//! - `Ordering::Acquire`：读操作，防止后续的读/写被重排到本操作之前
//! - `Ordering::Release`：写操作，防止之前的读/写被重排到本操作之后
//! - `Ordering::AcqRel`：同时具备 Acquire 和 Release 语义
//! - `Ordering::SeqCst`：顺序一致（全局统一排序）
//!
//! ## Release-Acquire 配对
//! 当线程 A 用 Release 写、线程 B 用 Acquire 读同一位置时，
//! 线程 B 能看到线程 A 在 Release 之前执行的所有写入。

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// 用 Release-Acquire 语义在两个线程之间安全地传递数据。
///
/// `produce` 先写数据，再用 Release 设置标志位；
/// `consume` 用 Acquire 读标志位，确保能看到数据。
pub struct FlagChannel {
    data: AtomicU32,
    ready: AtomicBool,
}

impl FlagChannel {
    pub const fn new() -> Self {
        Self {
            data: AtomicU32::new(0),
            ready: AtomicBool::new(false),
        }
    }

    /// 生产者：先存数据，再设置 ready 标志。
    ///
    /// TODO：选择正确的 Ordering
    /// - 写 data 应该用什么 Ordering？
    /// - 写 ready 应该用什么 Ordering？（确保 data 的写入对消费者可见）
    pub fn produce(&self, value: u32) {
        // TODO：存 data（选择合适的 Ordering）
        // TODO：设 ready = true（选择合适的 Ordering，确保 data 的写入先于它完成）
        self.data.store(value,Ordering::Relaxed);
        self.ready.store(true,Ordering::Release);
    }

    /// 消费者：自旋等待 ready 标志，然后读数据。
    ///
    /// TODO：选择正确的 Ordering
    /// - 读 ready 应该用什么 Ordering？（确保能看到 produce 写入的 data）
    /// - 读 data 应该用什么 Ordering？
    pub fn consume(&self) -> u32 {
        // TODO：自旋等待 ready 变为 true（选择合适的 Ordering）
        // TODO：读 data（选择合适的 Ordering）\
        while !self.ready.load( Ordering::Acquire){
            // core::hint::spin_loop();
        }
        
        self.data.load( Ordering::Relaxed)
    }


    /// 重置通道状态
    pub fn reset(&self) {
        self.ready.store(false, Ordering::Relaxed);
        self.data.store(0, Ordering::Relaxed);
    }
}

/// 一个用 SeqCst 实现的简单一次性初始化器。
/// 保证 `init` 只执行一次，且所有线程都能看到初始化后的值。
pub struct OnceCell {
    initialized: AtomicBool,
    value: AtomicU32,
}

impl OnceCell {
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
            value: AtomicU32::new(0),
        }
    }

    /// 尝试初始化。如果尚未初始化，存入值并返回 true。
    /// 如果已经初始化，返回 false。
    ///
    /// 提示：用 `compare_exchange` 确保只有一个线程成功。
    pub fn init(&self, val: u32) -> bool {
        // 先抢初始化权：只有赌赢的人才有资格写值
        // （输家绝不能碰 value 槽位，否则会覆盖赢家的数据）
        if self
            .initialized
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.value.store(val, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// 获取值。已初始化返回 Some，否则返回 None。
    pub fn get(&self) -> Option<u32> {
        // TODO：检查 initialized 标志，然后读值
          if self.initialized.load(Ordering::Acquire) {   // 先看标志（Acquire 拆箱）
          Some(self.value.load(Ordering::Relaxed))    // 再读数据
      } else {
          None
      }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_flag_channel() {
        let ch = Arc::new(FlagChannel::new());
        let ch2 = Arc::clone(&ch);

        let producer = thread::spawn(move || {
            ch2.produce(42);
        });

        let consumer = thread::spawn(move || ch.consume());

        producer.join().unwrap();
        let val = consumer.join().unwrap();
        assert_eq!(val, 42);
    }

    #[test]
    fn test_flag_channel_large_value() {
        let ch = Arc::new(FlagChannel::new());
        let ch2 = Arc::clone(&ch);

        let producer = thread::spawn(move || {
            ch2.produce(0xDEAD_BEEF);
        });

        let val = ch.consume();
        producer.join().unwrap();
        assert_eq!(val, 0xDEAD_BEEF);
    }

    #[test]
    fn test_once_cell_init_once() {
        let cell = OnceCell::new();
        assert!(cell.init(42));
        assert!(!cell.init(100));
        assert_eq!(cell.get(), Some(42));
    }

    #[test]
    fn test_once_cell_not_initialized() {
        let cell = OnceCell::new();
        assert_eq!(cell.get(), None);
    }

    #[test]
    fn test_once_cell_concurrent() {
        let cell = Arc::new(OnceCell::new());
        let mut handles = vec![];

        for i in 0..10 {
            let c = Arc::clone(&cell);
            handles.push(thread::spawn(move || c.init(i)));
        }

        let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        // 恰好只有一个线程初始化成功
        assert_eq!(results.iter().filter(|&&r| r).count(), 1);
        assert!(cell.get().is_some());
    }
}
