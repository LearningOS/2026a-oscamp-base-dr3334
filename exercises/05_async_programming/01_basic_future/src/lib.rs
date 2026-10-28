//! # 手动实现 Future
//!
//! 在这个练习中，你将为自定义类型手动实现 `Future` trait，以理解异步运行时的核心机制。
//!
//! ## 概念
//! - `std::future::Future` trait
//! - `Poll::Ready` 与 `Poll::Pending`
//! - `Waker` 的作用：通知运行时再次 poll

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// 倒计时 Future：每次被 poll 时 count 减 1，
/// count 归 0 时返回 `"liftoff!"`。
pub struct CountDown {
    pub count: u32,
}

impl CountDown {
    pub fn new(count: u32) -> Self {
        Self { count }
    }
}

// TODO：为 CountDown 实现 Future trait
// - Output 关联类型为 &'static str
// - 每次 poll：若 count == 0，返回 Poll::Ready("liftoff!")
// - 否则 count -= 1，调用 cx.waker().wake_by_ref()，返回 Poll::Pending
//
// 提示：用 `self.get_mut()` 拿到 `&mut Self`（因为 self 是 Pin<&mut Self>）
impl Future for CountDown {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let c = self.get_mut();
        match c.count {
            0 => Poll::Ready("liftoff!"),
            _ => {
                c.count -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

/// 只让出一次的 Future：第一次 poll 返回 Pending，第二次返回 Ready(())。
/// 这是最小的异步状态机示例。
pub struct YieldOnce {
    yielded: bool,
}

impl YieldOnce {
    pub fn new() -> Self {
        Self { yielded: false }
    }
}

// TODO：为 YieldOnce 实现 Future trait
// - Output 关联类型为 ()
// - 第一次 poll：置 yielded = true，唤醒 waker，返回 Pending
// - 第二次 poll：返回 Ready(())
impl Future for YieldOnce {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this=self.get_mut();
        if !this.yielded{
            this.yielded=true;
               cx.waker().wake_by_ref();
              Poll::Pending                // 第一次问：还没好
          } else {
              Poll::Ready(())    
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_countdown_zero() {
        let result = CountDown::new(0).await;
        assert_eq!(result, "liftoff!");
    }

    #[tokio::test]
    async fn test_countdown_three() {
        let result = CountDown::new(3).await;
        assert_eq!(result, "liftoff!");
    }

    #[tokio::test]
    async fn test_yield_once() {
        YieldOnce::new().await;
    }

    #[tokio::test]
    async fn test_countdown_large() {
        let result = CountDown::new(100).await;
        assert_eq!(result, "liftoff!");
    }
}
