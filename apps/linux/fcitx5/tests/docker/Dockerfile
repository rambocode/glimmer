# 微明 Fcitx5 插件的端到端测试环境：fcitx5（带 dbus 前端）+ 插件开发包 + cmake / g++ + PyGObject（Gio D-Bus 客户端）+ Rust 工具链。
# 源码不进镜像，由 run.sh 以 volume 挂到 /work。
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends -o Acquire::Retries=5 \
        fcitx5 fcitx5-modules libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev \
        cmake g++ \
        dbus python3 python3-gi gir1.2-glib-2.0 \
        build-essential pkg-config libssl-dev ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.96.0 \
    && chmod -R a+rwX /opt/cargo /opt/rustup

# dbus 需要 machine-id。
RUN dbus-uuidgen > /etc/machine-id

WORKDIR /work
