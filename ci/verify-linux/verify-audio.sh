#!/usr/bin/env bash
# Linux 検証コンテナのエントリポイント。
# サウンドサーバ（PulseAudio null sink / PipeWire loopback）を起動して
# からサブコマンドを実行する。`shell` で対話シェル、`none` でデーモン
# 無し起動（host_unavailable 経路の観測用）。
set -u

mode="${1:-shell}"
if [ "$mode" != "none" ] && [ "$mode" != "shell" ]; then
    shift || true
fi

start_pulse() {
    mkdir -p /run/pulse /run/dbus
    if [ ! -S /run/dbus/system_bus_socket ]; then
        dbus-daemon --system --fork --nopidfile 2>/dev/null || true
    fi
    pulseaudio --system --disallow-exit --disallow-module-loading=false \
        -D --exit-idle-time=-1 -L 'module-native-protocol-unix auth-anonymous=1' \
        --log-level=notice 2>/dev/null || true
    for _ in $(seq 1 30); do
        PULSE_SERVER=unix:/run/pulse/native pactl info >/dev/null 2>&1 && break
        sleep 0.3
    done
}

start_pipewire() {
    export XDG_RUNTIME_DIR=/run/xdg
    mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
    # session バスが無いと wireplumber が起動しないため用意する
    if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ]; then
        eval "$(dbus-launch --sh-syntax)"
    fi
    pipewire > /tmp/pipewire.log 2>&1 &
    for _ in $(seq 1 30); do
        [ -S "$XDG_RUNTIME_DIR/pipewire-0" ] && break
        sleep 0.3
    done
    # session manager 不在では stream ノードが link されず data
    # callback が発火しない（開通確認実測済み）。wireplumber を
    # 起動して自動 link を担わせる。
    wireplumber > /tmp/wireplumber.log 2>&1 &
    pw-loopback --capture-props='media.class=Audio/Sink node.name=verify_sink' \
        > /tmp/pw-loopback.log 2>&1 &
    sleep 2
}

case "$mode" in
    none)
        exec bash -l
        ;;
    pulse|shell)
        start_pulse
        ;;
    pipewire)
        start_pipewire
        ;;
    both)
        start_pulse
        start_pipewire
        ;;
esac

if [ "$mode" = "shell" ]; then
    exec bash -l
else
    exec "$@"
fi
