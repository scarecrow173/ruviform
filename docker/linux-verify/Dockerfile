# ruviform Linux 検証イメージ（Issue #953）
#
# 用途: ローカルの Linux ネイティブ検証 lane を docker 上で実行するための
# manual/opt-in 環境。CI の ubuntu-latest leg と同じ入口（mise + MISE_ENV=ci
# + `mise install --locked`）で toolchain を再現する。
#
# ビルド:
#   docker build -f docker/linux-verify/Dockerfile -t ruviform-linux-verify docker/linux-verify
#   （または `mise run verify-docker-build`）
#
# 実行手順と既存 CI との役割分担は evidence/rf953/ を参照。

FROM ubuntu:24.04

# mise 本体の pin。sha256 は upstream SHASUMS256.txt の公開値と照合する。
# 更新時は release の SHASUMS256.txt から linux-x64.tar.gz の値を転記する。
ARG MISE_VERSION=2026.11.2
ARG MISE_SHA256=c914dc3be1ad3588c3a7fa8ddae86c65e77eaf1777aa4a900c215e467f4c94a4

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
        # 基礎 toolchain + rsync（worktree → /work への同期）
        ca-certificates curl git rsync xz-utils \
        build-essential pkg-config \
        # verify lane 前提: cpal/alsa-sys build script が pkg-config 経由で要求
        libasound2-dev \
        # ui-gpui-verify / ui-gpui-offscreen-verify lane 依存
        # （ci.yml 各 install step が正本）
        libx11-dev libxi-dev libxtst-dev libwayland-dev wayland-protocols \
        libfontconfig1-dev libfreetype-dev libvulkan-dev libegl-dev \
        libgl1-mesa-dev mesa-vulkan-drivers libvulkan1 \
        xvfb xauth openbox x11-utils fonts-dejavu-core fontconfig \
    && rm -rf /var/lib/apt/lists/*

# mise バイナリを pin + sha256 照合で導入。rust 等のツールはイメージに
# 焼き込まず、実行時に entrypoint が mount されたリポジトリの
# mise.toml/mise.ci.toml から `mise install --locked` で解決する
# （pin 更新でイメージが陳腐化しない構成）。
RUN curl -fsSL "https://github.com/jdx/mise/releases/download/v${MISE_VERSION}/mise-v${MISE_VERSION}-linux-x64.tar.gz" -o /tmp/mise.tar.gz \
    && echo "${MISE_SHA256}  /tmp/mise.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/mise.tar.gz -C /usr/local/bin --strip-components=2 mise/bin/mise \
    && rm /tmp/mise.tar.gz \
    && mise --version

COPY entrypoint.sh /usr/local/bin/linux-verify
RUN chmod +x /usr/local/bin/linux-verify

# CI と同一モード: mise.ci.toml の rust pin（rust 1.99.0 + rustfmt/clippy）
# と Cargo.toml [profile.ci] が *-full タスクで有効化される。
ENV MISE_ENV=ci

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/linux-verify"]
CMD ["fmt-check"]
