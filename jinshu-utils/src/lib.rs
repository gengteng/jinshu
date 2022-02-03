//! 锦书工具函数库
//!

mod error;
pub mod secret;

pub use error::*;
use std::future::Future;
use std::net::IpAddr;
use std::time::SystemTime;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// 获取当前时间距离 1970-01-01 00:00:00 UTC 的毫秒数
///
/// # Panics
///
/// 如果计算机时钟比 1970-01-01 00:00:00 UTC 更早，会导致 panic。
///
pub fn current_millisecond() -> u64 {
    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Clock may have gone backwards");
    duration.as_millis() as u64
}

/// 获取当前时间距离 1970-01-01 00:00:00 UTC 的秒数
///
/// # Panics
///
/// 如果计算机时钟比 1970-01-01 00:00:00 UTC 更早，会导致 panic。
///
pub fn current_second() -> u64 {
    let now = SystemTime::now();
    let duration = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Clock may have gone backwards");
    duration.as_millis() as u64 / 1000
}

/// 获取本机所有网卡的非回环 ip 地址
///
pub fn get_all_ip_addr() -> std::io::Result<Vec<IpAddr>> {
    Ok(if_addrs::get_if_addrs()?
        .iter()
        .filter_map(|i| {
            let addr = i.addr.ip();
            if addr.is_loopback() {
                None
            } else {
                Some(addr)
            }
        })
        .collect::<Vec<_>>())
}

/// 监听 SIGINT（Ctrl-C） / SIGTERM
pub async fn shutdown_signal() {
    let sig_int = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let sig_term = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sig_term = std::future::pending::<()>();

    tokio::select! {
        _ = sig_int => println!("\r"),
        _ = sig_term => {},
    }
}

/// 用于关闭一个异步任务
///
/// # Example
///
/// ```no_run
/// use jinshu_utils::Keeper;
/// use tokio::time::sleep;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let keeper = Keeper::make(|mut waiter| async move {
///         loop {
///             tokio::select! {
///                 _ = &mut waiter => {
///                     break;
///                 }
///                 _ = sleep(Duration::from_secs(1)) => {
///                     println!("tick");
///                 }           
///             }
///         }
///
///         true
///     });
///
///     assert!(matches!(keeper.close().await, Ok(true)));
/// }
///
/// ```
///
pub struct Keeper<R> {
    closer: oneshot::Sender<()>,
    result_handle: JoinHandle<R>,
}

impl<R> Keeper<R>
where
    R: Send + 'static,
{
    /// 开始执行异步任务 `f` 并构造控制任务关闭的 `Keeper` 对象
    ///
    pub fn make<Fut, F>(f: F) -> Self
    where
        Fut: Future<Output = R> + Send + 'static,
        F: FnOnce(oneshot::Receiver<()>) -> Fut,
    {
        let (closer, waiter) = oneshot::channel();
        let result_handle = tokio::spawn(f(waiter));
        Self {
            closer,
            result_handle,
        }
    }

    /// 关闭异步任务并获得返回结果
    ///
    pub async fn close(self) -> Result<R> {
        let Keeper {
            closer,
            result_handle,
        } = self;

        closer
            .send(())
            .map_err(|_| Error::Other("The loop has already exited.".into()))?;

        Ok(result_handle.await?)
    }

    /// 判断是否已关闭
    ///
    pub fn is_closed(&self) -> bool {
        self.closer.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::Keeper;
    use crate::{current_millisecond, current_second, get_all_ip_addr};

    #[test]
    fn timestamp() {
        let cur = current_millisecond();
        assert_ne!(cur, 0);

        let cur = current_second();
        assert_ne!(cur, 0);
    }

    #[test]
    fn get_ips() {
        assert!(get_all_ip_addr().is_ok());
    }

    #[tokio::test]
    async fn keeper() {
        let keeper = Keeper::make(|waiter| async move {
            if let Err(_e) = waiter.await {
                // ignore
            }

            true
        });

        assert!(!keeper.is_closed());
        assert!(matches!(keeper.close().await, Ok(true)));

        let keeper = Keeper::make(|_waiter| async move { true });
        assert!(keeper.is_closed());
        assert!(keeper.close().await.is_err());
    }
}
