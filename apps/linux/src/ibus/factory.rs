//! `org.freedesktop.IBus.Factory`（ibus `src/ibusfactory.c`）：daemon 要用本输入法时调 `CreateEngine(s) -> o`，
//! 我们在 `/org/freedesktop/IBus/Engine/<n>` 导出一个新引擎。

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::Mutex;
use zbus::object_server::ObjectServer;
use zbus::zvariant::OwnedObjectPath;
use zbus::{fdo, interface};

use super::engine::Engine;
use super::service::Service;
use crate::backend::SharedBackend;
use crate::frontend::Session;

/// 工厂对象的路径。
pub const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

/// 引擎工厂：所有引擎共用一个后端。
pub struct Factory {
    /// 后端。
    backend: SharedBackend,

    /// 下一个引擎的编号。
    next: AtomicU32,
}

impl Factory {
    /// 用一个后端建工厂。
    pub fn new(backend: SharedBackend) -> Self {
        Self {
            backend,
            next: AtomicU32::new(1),
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl Factory {
    /// 建一个引擎实例，返回它的对象路径。组件里只登记了一个引擎，名字不做区分。
    async fn create_engine(
        &self,
        name: String,
        #[zbus(object_server)] server: &ObjectServer,
    ) -> fdo::Result<OwnedObjectPath> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let path = format!("/org/freedesktop/IBus/Engine/{id}");
        let session = Arc::new(Mutex::new(Session::new(Arc::clone(&self.backend))));
        server
            .at(path.as_str(), Engine::new(Arc::clone(&session)))
            .await?;
        server.at(path.as_str(), Service::new(session)).await?;
        tracing::info!(%name, %path, "创建引擎");
        OwnedObjectPath::try_from(path).map_err(|error| fdo::Error::Failed(error.to_string()))
    }
}
