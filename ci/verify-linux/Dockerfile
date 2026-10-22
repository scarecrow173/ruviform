# Ruviform Linux 検証コンテナ（Issue #60/#55/#57 の環境待ち項目）
#
# 使い分け:
#   - ALSA/PipeWire オーディオ実検証（#60）: PulseAudio null sink +
#     PipeWire loopback をデーモンとして起動し、cpal の ALSA /
#     pipewire 両 host 経路を観測する
#   - aarch64 NEON 計測（#55）: `docker buildx --platform linux/arm64`
#     で QEMU ユーザーエミュレーション下に実行
#   - UI PoC（#57）: WebKitGTK + Xvfb の headless 検証
#
# docker build -t ruviform-verify-linux ci/verify-linux
# （arm64 は docker buildx build --platform linux/arm64 …）

FROM rust:1-trixie

ENV DEBIAN_FRONTEND=noninteractive \
    PULSE_RUNTIME_PATH=/run/pulse

RUN apt-get update && apt-get install -y --no-install-recommends \
        # ビルド基盤
        build-essential pkg-config cmake git curl ca-certificates \
        # ALSA + PulseAudio（cpal default host / null-sink 検証）
        libasound2-dev libasound2-plugins pulseaudio \
        # PipeWire（cpal pipewire-backend feature）。bindgen が
        # libclang を必要とするため llvm/clang dev パッケージを入れる。
        # session manager（wireplumber）が無いと stream ノードが
        # link されずコールバックが発火しないため同梱する。
        pipewire libpipewire-0.3-dev pipewire-pulse wireplumber \
        dbus dbus-x11 \
        llvm-dev libclang-dev clang \
        # wry UI PoC（WebKitGTK 4.1 + headless X）
        libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev xvfb \
        xauth fonts-liberation \
        # ホスト Windows 側で buildx/QEMU が有効なら linux/arm64 で
        # そのまま動く（qemu-user binfmt 透過実行）
        qemu-user \
    && rm -rf /var/lib/apt/lists/*

# ALSA 既定デバイス → PulseAudio（cpal ALSA host の `default` が
# PA null sink に届く）。カード非存在環境でも ALSA init は成功する
# （pulse plugin 経路）。host_unavailable 経路は実行時に環境変数で
# 分岐して観測する。
COPY asound.conf /etc/asound.conf

# PipeWire: session 無しで daemon を直接起動し、loopback ノードで
# 仮想 duplex デバイスを作るための実行スクリプトを同梱する。
COPY verify-audio.sh /usr/local/bin/verify-audio.sh
RUN chmod +x /usr/local/bin/verify-audio.sh && \
    mkdir -p /run/pulse /run/dbus && \
    # PulseAudio は root 実行を拒否するため system モード運用とし、
    # 既定シンクを null sink に固定する設定を追記する
    { echo 'load-module module-null-sink sink_name=verify_null sink_properties=device.description=VerifyNull'; \
      echo 'set-default-sink verify_null'; } \
        >> /etc/pulse/system.pa && \
    # コンテナでは SHM/クリーンシャットダウン関係のモジュールが不安定
    # なため最小構成に寄せる。既存の native-protocol 行は pulse-access
    # グループ認証を要求するため anonymous に置き換える。
    sed -i 's/^load-module module-console-kit$//; s/^load-module module-systemd-login$//; s/^load-module module-udev-detect$//; s/^load-module module-detect$//; s/^load-module module-native-protocol-unix$/load-module module-native-protocol-unix auth-anonymous=1/' \
        /etc/pulse/system.pa

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/verify-audio.sh"]
CMD ["shell"]
