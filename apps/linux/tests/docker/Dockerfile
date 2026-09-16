# 微明 Linux IBus 引擎的端到端测试环境：ibus-daemon + dbus + PyGObject 驱动的 IBus 客户端 + Rust 工具链。
# 源码不进镜像，由 run.sh 以 volume 挂到 /work。
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ibus dbus dbus-x11 python3 python3-gi gir1.2-ibus-1.0 gir1.2-glib-2.0 \
        build-essential pkg-config libssl-dev ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.96.0 \
    && rustup component add rustfmt clippy --toolchain 1.96.0 \
    && chmod -R a+rwX /opt/cargo /opt/rustup

# dbus 需要 machine-id，ibus 也靠它拼私有总线地址文件名。
RUN dbus-uuidgen > /etc/machine-id

WORKDIR /work
