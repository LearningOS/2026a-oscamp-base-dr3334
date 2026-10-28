//! # Tokio 异步任务
//!
//! 在这个练习中，你将使用 `tokio::spawn` 创建并发的异步任务。
//!
//! ## 概念
//! - `tokio::spawn` 创建异步任务
//! - `JoinHandle` 等待任务完成
//! - 异步任务之间的并发执行

use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// 并发计算 0..n 中每个数的平方，收集结果并按顺序返回。
///
/// 提示：为每个 i 创建一个 `tokio::spawn` 任务，收集 JoinHandle，再逐个 await。
pub async fn concurrent_squares(n: usize) -> Vec<usize> {
    // TODO：创建 n 个异步任务，每个计算 i * i
    // TODO：收集所有 JoinHandle
    // TODO：逐个 await 拿到结果
    let mut handles = Vec::new();
    for i in 0..n{
        handles.push(tokio::spawn(async move { i * i }));
    }
     let mut results = Vec::new();
      for h in handles {
          results.push(h.await.unwrap());
      }
      results
}

/// 并发执行多个"耗时"任务（用 sleep 模拟），返回所有结果。
/// 每个任务睡 `duration_ms` 毫秒后返回自己的 `task_id`。
///
/// 关键：所有任务应并发执行，总耗时应接近单个任务的耗时，而不是所有任务耗时之和。
pub async fn parallel_sleep_tasks(n: usize, duration_ms: u64) -> Vec<usize> {
    // TODO：为 0..n 中的每个 id 创建异步任务
    // TODO：每个任务睡指定时长后返回自己的 id
    // TODO：收集所有结果并排序
     let mut handles = Vec::new();
      for id in 0..n {
          handles.push(tokio::spawn(async move {
              sleep(Duration::from_millis(duration_ms)).await;   // 模拟耗时操作
              id                                                  // 返回自己的编号
          }));
      }

      let mut results = Vec::new();
      for h in handles {
          results.push(h.await.unwrap());
      }
      results.sort();   // hint 要求排序（其实按凭证收已有序，但保险）
      results
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Instant;

    #[tokio::test]
    async fn test_squares_basic() {
        let result = concurrent_squares(5).await;
        assert_eq!(result, vec![0, 1, 4, 9, 16]);
    }

    #[tokio::test]
    async fn test_squares_zero() {
        let result = concurrent_squares(0).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_squares_one() {
        let result = concurrent_squares(1).await;
        assert_eq!(result, vec![0]);
    }

    #[tokio::test]
    async fn test_parallel_sleep() {
        let start = Instant::now();
        let result = parallel_sleep_tasks(5, 100).await;
        let elapsed = start.elapsed();

        assert_eq!(result, vec![0, 1, 2, 3, 4]);
        // 并发执行时，总耗时应远小于 5 * 100ms
        assert!(
            elapsed.as_millis() < 400,
            "任务应并发执行，实际耗时 {}ms",
            elapsed.as_millis()
        );
    }
}
