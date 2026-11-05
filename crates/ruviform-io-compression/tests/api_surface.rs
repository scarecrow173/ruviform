//! Public-API surface contract: backend hiding, feature shape, and
//! thread-safety bounds. What cannot appear in signatures is enforced
//! structurally (no backend type is importable through this crate);
//! these tests pin the properties that *are* observable.

#![cfg(any(feature = "gzip", feature = "deflate"))]

mod common;

use ruviform_io_compression::{
    CompressionError, CompressionErrorKind, DecodeLimits, DecodeScope, EncodeLimits,
};

#[test]
fn error_is_send_sync_and_preserves_kind_through_io() {
    fn bounds<T: Send + Sync + 'static>() {}
    bounds::<CompressionError>();

    let error = CompressionError::from_kind(CompressionErrorKind::TrailingData);
    let io_error: std::io::Error = error.into();
    assert_eq!(
        common::compression_kind(&io_error),
        Some(CompressionErrorKind::TrailingData),
        "kind must survive the io::Error boundary for Read/Write callers"
    );
}

#[test]
fn limits_default_to_unbounded() {
    let encode = EncodeLimits::new();
    assert_eq!(encode.max_input_bytes(), u64::MAX);
    assert_eq!(encode.max_output_bytes(), u64::MAX);
    let decode = DecodeLimits::new();
    assert_eq!(decode.max_compressed_bytes(), u64::MAX);
    assert_eq!(decode.max_decoded_bytes(), u64::MAX);
    assert_eq!(decode.max_units(), u64::MAX);
}

#[test]
fn scope_defaults_to_whole_input() {
    assert_eq!(DecodeScope::default(), DecodeScope::WholeInput);
}

#[test]
fn public_types_are_send_when_borrowed_stream_is_send() {
    #[cfg(feature = "gzip")]
    {
        fn bounds<T: Send>() {}
        bounds::<ruviform_io_compression::gzip::Encoder<'_, Vec<u8>>>();
        bounds::<ruviform_io_compression::gzip::Decoder<'_, std::io::BufReader<&[u8]>>>();
    }
    #[cfg(feature = "deflate")]
    {
        fn bounds<T: Send>() {}
        bounds::<ruviform_io_compression::deflate::Encoder<'_, Vec<u8>>>();
        bounds::<ruviform_io_compression::deflate::Decoder<'_, std::io::BufReader<&[u8]>>>();
    }
}
