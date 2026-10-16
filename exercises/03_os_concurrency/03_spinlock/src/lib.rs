//! # 自旋锁（Spin Lock）
//!
//! 在这个练习中，你将实现一个基础的自旋锁。
//! 自旋锁是 OS 内核中最基础的同步原语之一。
//!
//! ## 关键概念
//! - 自旋锁通过忙等待（busy-waiting）来获取锁
//! - `AtomicBool` 的 `compare_exchange` 实现加锁
//! - `core::hint::spin_loop` 降低 CPU 功耗
//! - `UnsafeCell` 提供内部可变性

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};

/// 基础自旋锁
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// 获取锁，返回内部数据的可变引用。
    ///
    /// TODO：用 compare_exchange 自旋直到拿到锁
    /// 1. 循环中尝试把 locked 从 false 改成 true
    /// 2. 成功用 Acquire 顺序，失败用 Relaxed
    /// 3. 失败时调用 `core::hint::spin_loop()` 给 CPU 一个提示
    /// 4. 成功后返回 `&mut *self.data.get()`
    ///
    /// # 安全性
    /// 调用者必须保证用完数据后调用 `unlock`。
    pub fn lock(&self) -> &mut T {
        // TODO
        loop{
            match self.locked.compare_exchange(false,true,Ordering::Acquire,Ordering::Relaxed){
                Ok(_) =>  return unsafe { &mut *self.data.get() }  ,
                Err(_) => core::hint::spin_loop(),
            }
        }
    }

    /// 释放锁。
    ///
    /// TODO：把 locked 设为 false（使用 Release 顺序）
    pub fn unlock(&self) {
        // TODO
         self.locked.store(false, Ordering::Release);
    }

    /// 尝试获取锁但不自旋。
    /// 成功返回 Some(&mut T)，锁忙返回 None。
    pub fn try_lock(&self) -> Option<&mut T> {
        // TODO：只做一次 compare_exchange 尝试
        match self.locked.compare_exchange(
          false,
          true,
          Ordering::Acquire,
          Ordering::Relaxed,
      ) {
          Ok(_) => Some(unsafe { &mut *self.data.get() }),
          Err(_) => None,      // 锁忙：不等待，直接报告失败
      }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_basic_lock_unlock() {
        let lock = SpinLock::new(0u32);
        {
            let data = lock.lock();
            *data = 42;
            lock.unlock();
        }
        let data = lock.lock();
        assert_eq!(*data, 42);
        lock.unlock();
    }

    #[test]
    fn test_try_lock() {
        let lock = SpinLock::new(0u32);
        assert!(lock.try_lock().is_some());
        lock.unlock();
    }

    #[test]
    fn test_concurrent_counter() {
        let lock = Arc::new(SpinLock::new(0u64));
        let mut handles = vec![];

        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                for _ in 0..1000 {
                    let data = l.lock();
                    *data += 1;
                    l.unlock();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let data = lock.lock();
        assert_eq!(*data, 10000);
        lock.unlock();
    }

    #[test]
    fn test_lock_protects_data() {
        let lock = Arc::new(SpinLock::new(Vec::new()));
        let mut handles = vec![];

        for i in 0..5 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let data = l.lock();
                data.push(i);
                l.unlock();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let data = lock.lock();
        let mut sorted = data.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
        lock.unlock();
    }
}
