//! `org.freedesktop.IBus.Service`（ibus `src/ibusservice.c`）：每个 IBus 对象都有的 `Destroy`，daemon 释放引擎时调。

use std::sync::Arc;

use tokio::sync::Mutex;
use zbus::message::Header;
use zbus::{Connection, interface};

use super::engine::Engine;
use crate::frontend::Session;

/// 与引擎挂在同一路径上的 `Service` 接口。
pub struct Service {
    /// 同路径引擎的前端状态，销毁时关协议会话。
    session: Arc<Mutex<Session>>,
}

impl Service {
    /// 绑定一个引擎的前端状态。
    pub fn new(session: Arc<Mutex<Session>>) -> Self {
        Self { session }
    }
}

#[interface(name = "org.freedesktop.IBus.Service")]
impl Service {
    /// 关会话，把本路径上的引擎与本接口从对象服务器上摘掉。
    /// 摘除放到另一个任务里做：正在分派本方法时改对象树，不指望对象服务器的锁能重入。
    async fn destroy(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) {
        self.session.lock().await.close();
        let Some(path) = header.path().map(|path| path.to_owned().into_owned()) else {
            return;
        };
        tracing::info!(%path, "销毁引擎");
        let connection = connection.clone();
        tokio::spawn(async move {
            let server = connection.object_server();
            if let Err(error) = server.remove::<Engine, _>(&path).await {
                tracing::warn!(%error, "摘除引擎失败");
            }
            if let Err(error) = server.remove::<Service, _>(&path).await {
                tracing::warn!(%error, "摘除 Service 失败");
            }
        });
    }
}
