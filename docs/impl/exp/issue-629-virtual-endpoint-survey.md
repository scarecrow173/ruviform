# Issue #629 virtual endpoint probe — experiments

Issue #629 first slice の probe 実装中に観測した実機挙動。環境: Windows 11 10.0.26300.9974, x86_64, MINGW64 shell。

## E1: `IsFormatSupported` の `ppclosestmatch` に NULL を渡すと `E_POINTER`

- 観測: `IAudioClient::IsFormatSupported(eRender, &mut format, std::ptr::null_mut())` が `E_POINTER` を返した（GetMixFormat 由来の valid format でも）。
- 有効な out-pointer（`*mut *mut WAVEFORMATEX`）を渡すと `S_OK`（または `AUDCLNT_E_UNSUPPORTED_FORMAT` + closest format）を返した。
- 採用: probe は常に有効な out-param を渡し、返却された closest format を `CoTaskMemFree` で解放する。
- 意義: WASAPI のこのメソッドは「サポート外でも closest を返す」設計であり、NULL out-param を拒否する実装がある。probe 以外の呼出し側でも同じ規約が必要。

## E2: `midiOutMessage` `DRV_QUERYDEVICEINTERFACE*` が segfault

- 観測: `midiOutMessage(dev, DRV_QUERYDEVICEINTERFACESIZE, ...)` が rc=0, size=2 を返した後、`DRV_QUERYDEVICEINTERFACE` の fetch 呼出しが WinMM 内でプロセスを落とした（対象: Microsoft GS Wavetable Synth, output index 0）。
- 対照: input 側（`midiInMessage` + `DRV_QUERYDEVICEINTERFACE*`）は adapter `src/os/windows.rs` の先例どおり安全に動作。ただし本ホストは midi input 0 件のため実 fetch 経路は未検証。
- 採用: MIDI probe は output 側 interface-id query を行わず `NotRun` + note で報告。`midiOutOpen`/`midiOutShortMsg`/`midiOutLongMsg` も呼ばない。
- 意義: WinMM の interface-id 機構は device/driver 依存で crash し得る。output 側の stable id 取得は後続 slice で別手段（SetupAPI 等）を検討するか、安全性を実 provisioning 環境で再検証する。

## E3: not-present / unplugged endpoint は enumerate されるが activate 失敗

- 観測: audio probe が 61 endpoint を列挙（本機は物理 device が少なく、多くは stale）。`state != DEVICE_STATE_ACTIVE` の endpoint で `Activate<IAudioClient>` が `E_NOTFOUND` 系 HRESULT を返した。
- 採用: activate 失敗は行ごとの note に記録して継続し、endpoint 自体は state 付きで報告する。
- 意義: 仮想 endpoint provisioning 判定では「列挙に現れる」だけでは不十分で、activate 成功が実用性の下限。ただし activate 成功も streaming 証明ではない（`virtual-os-endpoint` evidence ではない）。

## E4: `GetStreamLatency` は `IAudioClient::Initialize` 前に `AUDCLNT_E_NOT_INITIALIZED`

- 観測: Initialize を呼ばない probe では `GetStreamLatency` が `AUDCLNT_E_NOT_INITIALIZED` を返す。
- 採用: latency 値を推測せず `NotRun` として報告。streaming latency の実測は次 slice（runtime controller）で Initialize 後に行う。
- 意義: 「query-only で取れる値」と「stream 必須の値」の境界を probe 出力で明示する例。

## E5: production `backend().input_ports()` は WinMM input-only を反映

- 観測: MIDI probe の port view で `backend().input_ports()` は input port のみを列挙し、`MidiPortOrigin` は `Unknown` を返す。output 側の virtual 検出は production API 面には存在しない。
- 意義: loopback send→receive 検証には `midiOut*` 送信経路の adapter 追加が後続 slice で必要（discovery doc のギャップとして記録）。
