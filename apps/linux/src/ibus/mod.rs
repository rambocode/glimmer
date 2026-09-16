//! IBus 的 D-Bus 层：找私有总线（[`address`]）、导出工厂与引擎对象、IBus 对象的变体编码（[`variant`]）。
//! 纯 Rust 走 zbus，不链接 libibus。

pub mod address;
pub mod variant;

mod engine;
mod factory;
mod service;

use zbus::connection::Builder;

use crate::backend::SharedBackend;
use crate::error::LinuxError;

pub use engine::Engine;
pub use factory::{FACTORY_PATH, Factory};
pub use service::Service;

/// 本输入法组件在 IBus 总线上的名字，与组件 XML 的 `<name>` 一致。
pub const BUS_NAME: &str = "app.glimmer.IBus";

/// 连上 `address` 指向的 IBus 总线，导出工厂、占住 [`BUS_NAME`]，一直服务到 daemon 断开连接。
pub async fn serve(backend: SharedBackend, address: &str) -> Result<(), LinuxError> {
    let connection = Builder::address(address)?
        .serve_at(FACTORY_PATH, Factory::new(backend))?
        .build()
        .await?;
    connection.request_name(BUS_NAME).await?;
    tracing::info!(address, name = BUS_NAME, "已连上 IBus 总线");
    connection.closed().await;
    tracing::info!("IBus 总线已断开，退出");
    Ok(())
}
