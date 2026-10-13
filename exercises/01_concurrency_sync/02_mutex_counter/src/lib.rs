//! # Mutex Shared State
//!
//! In this exercise, you will use `Arc<Mutex<T>>` to safely share and modify data between multiple threads.
//!
//! ## Concepts
//! - `Mutex<T>` mutex protects shared data
//! - `Arc<T>` atomic reference counting enables cross-thread sharing
//! - `lock()` acquires the lock and accesses data

use std::sync::{Arc, Mutex};
use std::thread;

/// Increment a counter concurrently using `n_threads` threads.
/// Each thread increments the counter `count_per_thread` times.
/// Returns the final counter value.
///
/// Hint: Use `Arc<Mutex<usize>>` as the shared counter.
pub fn concurrent_counter(n_threads: usize, count_per_thread: usize) -> usize {
    // TODO: Create Arc<Mutex<usize>> with initial value 0
    // TODO: Spawn n_threads threads
    // TODO: In each thread, lock() and increment count_per_thread times
    // TODO: Join all threads, return final value
    let counter = Arc::new(Mutex::new(0usize));
  
     let mut handles = Vec::new();
    for _ in 0..n_threads{
        let counter = Arc::clone(&counter);
        let handle=thread::spawn(move ||{
            for _ in 0..count_per_thread{
                let mut num=counter.lock().unwrap();
                *num+=1;
            }
           
        });
        handles.push(handle);

    }
       for h in handles {                                 // 循环结束后,统一等待所有线程
          h.join().unwrap();                             // join 每个子线程
      }
        let total = *counter.lock().unwrap();
        total
}
     
 
/// Add elements to a shared vector concurrently using multiple threads.
/// Each thread pushes its own id (0..n_threads) to the vector.
/// Returns the sorted vector.
///
/// Hint: Use `Arc<Mutex<Vec<usize>>>`.

  pub fn concurrent_collect(n_threads: usize) -> Vec<usize> {
      // ① 共享一个"空的 Vec",用 Arc<Mutex<..>> 包起来
      let shared = Arc::new(Mutex::new(Vec::new()));     // Arc<Mutex<Vec<usize>>>

      let mut handles = Vec::new();
      for id in 0..n_threads {                            // ← 用 id,不是 _
          let shared = Arc::clone(&shared);
          let handle = thread::spawn(move || {            // move 把 id 和句柄搬进去
              let mut guard = shared.lock().unwrap();      // 上锁
              guard.push(id);                          // ② 推什么进去?
          });
          handles.push(handle);
      }

      for h in handles {                                  // 等所有线程
          h.join().unwrap();
      }

      let mut result = shared.lock().unwrap();             // ③ 上锁取出
      result.sort();                                       
      result.clone()
                                           // ④ 返回一个 Vec<usize>
  }


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_single_thread() {
        assert_eq!(concurrent_counter(1, 100), 100);
    }

    #[test]
    fn test_counter_multi_thread() {
        assert_eq!(concurrent_counter(10, 100), 1000);
    }

    #[test]
    fn test_counter_zero() {
        assert_eq!(concurrent_counter(5, 0), 0);
    }

    #[test]
    fn test_collect() {
        let result = concurrent_collect(5);
        assert_eq!(result, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_collect_single() {
        assert_eq!(concurrent_collect(1), vec![0]);
    }
}
