//! 高精度 `f64` オフライン数値・DSP プリミティブ（Issue #150）。
//!
//! 用途非依存の FFT / Real FFT / 数値ベクトル / 窓関数 / polyphase リサンプラ /
//! overlap-save 畳込み / 最小二乗を提供する。メディア固有概念・I/O・スレッドは
//! 持たず、依存は std のみ（Primitives 最下位）。
//!
//! # 精度契約
//!
//! - すべての API は `f64` 専用。`f64` → `f32` の暗黙ダウングレードは存在しない。
//! - scalar 実装は逐次評価順を固定し、FMA 融合（`f64::mul_add`）を使わない
//!   `a*b+c` 形で統一する。結果は全プラットフォーム・全 ISA で bit-exact。
//! - denormal は flush しない（FTZ/DAZ はエンジン側責務）。
//! - NaN / Inf は IEEE どおり伝播させる。`corr0` は NaN を含むエネルギー縮退を
//!   `0.0` へ写像する。`linalg` は非有限入力を `None` で拒否する。
//!
//! # FFT 規約
//!
//! - 複素数は実部・虚部を分離した等長スライス（split re/im planar）で受け渡す。
//!   `re[k] + i·im[k]` が bin k の複素値。インターリーブ形式は提供しない。
//! - forward は負符号回転・正規化なし、inverse は正符号回転・`1/N` 正規化。
//! - `RealFftPlan::forward` の出力は非負周波数 `N/2+1` bin（k=0 が DC、k=N/2 が
//!   Nyquist）。負周波数は共役対称で省略する。`inverse` は同じ配置を入力に取り、
//!   DC / Nyquist の虚部は読まない。
//! - 変換長の調整・ゼロ埋めは呼び出し側の責務。API は入力長と plan の `n` の
//!   完全一致を要求し、内部で暗黙に延長・切詰めしない。
//! - plan（不変・共有可能）と workspace（`&mut` 排他）を分離し、変換処理中の
//!   確保を排除する。
//!
//! # エラー契約
//!
//! 公開 API は panic-free。検証失敗（長さ不一致・非 2 冪・レート 0 等）は
//! `Result<_, Error>`（`Error::InvalidValue`）で返す。`linalg` のみ「妥当な
//! 入力だが解が存在しない」（ランク落ち・非有限）を `Option` で表す。
//!
//! # RT 契約
//!
//! RT-suitable（alloc なし・有界・事前確保済み状態でのみ動作）な API と
//! non-RT（確保を伴う）API は関数単位で明記する。plan・workspace・係数表の
//! 構築はすべて non-RT であり、呼び出し側が事前に確保する。

pub mod conv;
mod error;
pub mod fft;
pub mod linalg;
pub mod resample;
pub mod vec;
pub mod window;

pub use error::Error;

/// `Result` alias（`Error` は本クレートの検証エラー型）。
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod rt_tests {
    use crate::{conv, fft, resample, vec, window};
    use assert_no_alloc::assert_no_alloc;

    /// Issue #150 RT-verification lane: `AllocDisabler` swaps the lib
    /// test binary's allocator so `assert_no_alloc` catches any
    /// allocation inside RT-suitable paths. One per crate test binary.
    #[global_allocator]
    static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

    /// plan / workspace / 係数表は non-RT の準備フェーズで構築し、
    /// RT-suitable と明記した処理 API が一切確保しないことを検証する。
    #[test]
    fn rt_processing_paths_do_not_allocate() {
        // 準備（確保を伴う non-RT フェーズ）。
        let plan = fft::FftPlan::new(1024).unwrap();
        let mut ws = fft::FftWorkspace::new(&plan);
        let mut re = vec![0.0; 1024];
        let mut im = vec![0.0; 1024];

        let rplan = fft::RealFftPlan::new(2048).unwrap();
        let mut rws = fft::RealFftWorkspace::new(&rplan);
        let input = vec![0.5; 2048];
        let mut sre = vec![0.0; 1025];
        let mut sim = vec![0.0; 1025];
        let mut time = vec![0.0; 2048];

        let rs = resample::ResamplePlan::new(48000, 96000).unwrap();
        let rin = vec![1.0; 64];
        let mut rout = vec![0.0; 128];

        let cp = conv::ConvPlan::new(&[0.25, 0.5, 0.25]).unwrap();
        let mut cws = conv::ConvWorkspace::new(&cp);
        let cin = vec![1.0; 3000];
        let mut cout = vec![0.0; 3000];

        let mut coeffs = [0.0; 16];
        let mut y = vec![1.0; 64];
        let x = vec![2.0; 64];
        let mut mag = [0.0; 64];

        assert_no_alloc(|| {
            vec::scale(&x, 0.5, &mut y).unwrap();
            vec::axpy(&mut y, 0.25, &x).unwrap();
            vec::sub_scaled(&mut y, 0.1, &x).unwrap();
            std::hint::black_box(vec::dot(&y, &x).unwrap());
            std::hint::black_box(vec::sum_sq(&y));
            std::hint::black_box(vec::corr0(&y, &x).unwrap());
            std::hint::black_box(vec::rms(&y));
            vec::mag_sq(&x, &y, &mut mag).unwrap();
            window::Window::Hann.fill(&mut coeffs);
            plan.transform(&mut ws, &mut re, &mut im, false).unwrap();
            rplan.forward(&mut rws, &input, &mut sre, &mut sim).unwrap();
            rplan.inverse(&mut rws, &sre, &sim, &mut time).unwrap();
            rs.resample_into(&rin, &mut rout).unwrap();
            cp.convolve(&mut cws, &cin, &mut cout).unwrap();
        });
    }
}
