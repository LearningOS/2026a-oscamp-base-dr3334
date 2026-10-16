//! # 读写锁（写者优先）
//!
//! 在这个练习中，你将仅用原子操作从零实现一个**写者优先**的读写锁。
//! 多个读者可以同时持有读锁；写者独占持有。
//!
//! **注意：** Rust 标准库已提供 [`std::sync::RwLock`]。本练习不用标准库版本，
//! 而是手写一个最小实现，用于学习协议和策略。
//!
//! ## 读写锁的常见策略
//! 当读者和写者都在等待时，不同实现可以给不同的**优先级**：
//!
//! - **读者优先（Reader-priority）**：写者等待期间新读者仍可进入，若读者源源不断，写者可能被饿死。
//! - **写者优先（Writer-priority）**：只要有写者在等待，新读者一律不放行，直到该写者执行完毕；本练习实现此策略。
//! - **读写公平（Read-write fair）**：按公平顺序（如 FIFO 或轮转）处理请求，读者和写者都不会被系统性饿死。
//!
//! ## 关键概念
//! - **读者（Readers）**：共享访问；多个线程可以同时持有读锁。
//! - **写者（Writer）**：独占访问；只允许一个写者，且写者持锁期间不允许任何读者。
//! - **写者优先（本实现）**：只要有一个写者在等待，新读者就阻塞，直到该写者运行。
//!
//! ## 状态（单个原子变量）
//! 我们用一个 `AtomicU32`：低位 = 读者计数，两个标志位 = 写者持锁 / 写者等待。
//! 所有逻辑都用 compare_exchange 和 load/store 实现；不使用 `std::sync::RwLock`。

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};

/// 并发读者的最大数量（要装得进 state 的位宽）。
const READER_MASK: u32 = (1 << 30) - 1;
/// 写者持锁时置位的比特。
const WRITER_HOLDING: u32 = 1 << 30;
/// 至少有一个写者在等待时置位的比特（写者优先：阻塞新读者）。
const WRITER_WAITING: u32 = 1 << 31;

/// 写者优先的读写锁。从零实现；不使用 `std::sync::RwLock`。
pub struct RwLock<T> {
    state: AtomicU32,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            data: UnsafeCell::new(data),
        }
    }

    /// 获取读锁。阻塞（自旋）直到没有写者持锁且没有写者等待（写者优先）。
    ///
    /// TODO：实现读锁获取
    /// 1. 循环中 load state（Acquire）。
    /// 2. 如果 WRITER_HOLDING 或 WRITER_WAITING 置位，spin_loop 后继续（写者优先：写者等待时不放新读者进来）。
    /// 3. 如果读者计数（state & READER_MASK）已达 READER_MASK，自旋后继续。
    /// 4. 尝试 compare_exchange(s, s + 1, AcqRel, Acquire)；成功返回 RwLockReadGuard { lock: self }。
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        // TODO
         loop {
          let s = self.state.load(Ordering::Acquire);
          // 持有锁 + 等待写检查  如果都没有，检查是否有读等待 
          if s & (WRITER_HOLDING | WRITER_WAITING) != 0 {   // 写者优先拦截
              core::hint::spin_loop();
              continue;
          }
          if s & READER_MASK == READER_MASK {               // 读者满了（防进位）
              core::hint::spin_loop();
              continue;
          }
          match self.state.compare_exchange(s, s + 1, Ordering::AcqRel, Ordering::Acquire) {
              Ok(_) => return RwLockReadGuard { lock: self },
              Err(_) => continue,
          }
      }
    }

    /// 获取写锁。阻塞直到没有读者且没有其他写者。
    ///
    /// TODO：实现写锁获取（写者优先）
    /// 1. 先置 WRITER_WAITING：fetch_or(WRITER_WAITING, Release)，让新读者阻塞。
    /// 2. 循环：load state；如果有读者（READER_MASK）或 WRITER_HOLDING，spin_loop 后继续。
    /// 3. 尝试 compare_exchange(WRITER_WAITING, WRITER_HOLDING, ...) 抢锁；或当写者刚释放时 compare_exchange(0, WRITER_HOLDING, ...)。
    /// 4. 成功返回 RwLockWriteGuard { lock: self }。
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        // TODO、
        // ⚠️ 关键点①（你来定）：宣言该用什么 Ordering？提示：它在"发布"信号
        self.state.fetch_or(WRITER_WAITING,Ordering::Release); // set flag bit
        loop{
            let s= self.state.load(Ordering::Acquire);
            if (s&READER_MASK!=0||s&WRITER_HOLDING!=0){
                core::hint::spin_loop();
              continue;
            }
            // ⚠️ 关键点②（你来修）：走到这里时合法局面只有两种——WRITER_WAITING 或 0
            //    （WRITER_HOLDING 在上面已经被 continue 拦走了）这个比较对象对吗？
            let res= if s==WRITER_WAITING{
                self.state.compare_exchange(WRITER_WAITING, WRITER_HOLDING, Ordering::AcqRel, Ordering::Acquire)
            }else{
                self.state.compare_exchange(0, WRITER_HOLDING, Ordering::AcqRel, Ordering::Acquire)
            };

            if res.is_ok() {
              return RwLockWriteGuard { lock: self };
          }
        }
        
    }
}

/// 读锁守卫；drop 时释放读锁。
pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

// TODO：为 RwLockReadGuard 实现 Deref
// 返回数据的共享引用：unsafe { &*self.lock.data.get() }
impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
         unsafe { &*self.lock.data.get() }
    }
}

// TODO：为 RwLockReadGuard 实现 Drop
// 读者计数减一：self.lock.state.fetch_sub(1, Ordering::Release)
impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

/// 写锁守卫；drop 时释放写锁。
pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

// TODO：为 RwLockWriteGuard 实现 Deref
// 返回共享引用：unsafe { &*self.lock.data.get() }
impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

// TODO：为 RwLockWriteGuard 实现 DerefMut
// 返回可变引用：unsafe { &mut *self.lock.data.get() }
impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

// TODO：为 RwLockWriteGuard 实现 Drop
// 清掉写者标志位让锁空闲：self.lock.state.fetch_and(!(WRITER_HOLDING | WRITER_WAITING), Ordering::Release)
impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_and(!(WRITER_HOLDING | WRITER_WAITING), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_multiple_readers() {
        let lock = Arc::new(RwLock::new(0u32));
        let mut handles = vec![];
        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let g = l.read();
                assert_eq!(*g, 0);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_writer_excludes_readers() {
        let lock = Arc::new(RwLock::new(0u32));
        let lock_w = Arc::clone(&lock);
        let writer = thread::spawn(move || {
            let mut g = lock_w.write();
            *g = 42;
        });
        writer.join().unwrap();
        let g = lock.read();
        assert_eq!(*g, 42);
    }

    #[test]
    fn test_concurrent_reads_after_write() {
        let lock = Arc::new(RwLock::new(Vec::<i32>::new()));
        {
            let mut g = lock.write();
            g.push(1);
            g.push(2);
        }
        let mut handles = vec![];
        for _ in 0..5 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let g = l.read();
                assert_eq!(g.len(), 2);
                assert_eq!(&*g, &[1, 2]);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_writes_serialized() {
        let lock = Arc::new(RwLock::new(0u64));
        let mut handles = vec![];
        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut g = l.write();
                    *g += 1;
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(*lock.read(), 1000);
    }
}
