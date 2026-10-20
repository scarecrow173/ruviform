//! Sample storage types: the sealed [`Sample`] trait.
//!
//! `f32` is the RT-canonical instantiation — every processing buffer
//! defaults to it ([`AudioBuffer`](crate::AudioBuffer)`<f32>`). `f64`
//! exists only because the design baseline permits a sample type
//! parameter; no further `f64` machinery (conversion, mixing) is
//! provided here.
//!
//! The trait is sealed: it is implemented for `f32` and `f64` only and
//! cannot be implemented downstream.

/// Implementation detail sealing [`Sample`]. Not nameable outside this
/// crate, so no downstream implementations are possible.
mod sealed {
    pub trait Sealed {}

    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

/// A PCM sample storage type.
///
/// Sealed — implemented for `f32` and `f64` only. `f32` is the
/// RT-canonical documented instantiation.
///
/// **RT contract:** RT-suitable — `Copy` scalar; nothing about the trait
/// allocates or blocks.
///
/// # Examples
///
/// ```
/// use ruviform_audio_core::Sample;
///
/// fn zeroed<S: Sample>(len: usize) -> Vec<S> {
///     vec![S::default(); len]
/// }
///
/// assert_eq!(f32::BYTES, 4);
/// assert_eq!(f64::BYTES, 8);
/// assert_eq!(zeroed::<f32>(3), [0.0; 3]);
/// ```
pub trait Sample: sealed::Sealed + Copy + Default + Send + Sync + 'static {
    /// Size of one sample in bytes — `size_of::<Self>()`.
    ///
    /// **RT contract:** RT-suitable — compile-time constant.
    const BYTES: usize;
}

impl Sample for f32 {
    const BYTES: usize = size_of::<f32>();
}

impl Sample for f64 {
    const BYTES: usize = size_of::<f64>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_match_scalar_sizes() {
        assert_eq!(f32::BYTES, size_of::<f32>());
        assert_eq!(f64::BYTES, size_of::<f64>());
    }

    #[test]
    fn generic_buffer_code_uses_default_as_zero() {
        fn zeroed<S: Sample>(len: usize) -> Vec<S> {
            vec![S::default(); len]
        }
        fn sum<S: Sample + core::ops::Add<Output = S>>(samples: &[S], init: S) -> S {
            samples.iter().copied().fold(init, |acc, s| acc + s)
        }
        // `Default` is the zeroed-fill contract used by `AudioBuffer::new`.
        assert_eq!(zeroed::<f32>(2), [0.0; 2]);
        assert_eq!(sum(&[1.0_f32, 2.0, 3.0], f32::default()), 6.0);
    }
}
