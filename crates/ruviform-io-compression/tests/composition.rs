//! `std::io` composition: the borrowed adapters must layer onto any
//! `Write`/`BufRead` — including `dyn` trait objects and a TAR-shaped
//! in-memory archive — without owning them.

#![cfg(all(feature = "gzip", feature = "deflate"))]

mod common;

use std::io::{BufReader, Read, Write};

use ruviform_io_compression::deflate;
use ruviform_io_compression::gzip;

/// Minimal TAR-style header builder — a fixed-size name field plus
/// octal size, mirroring the real 512-byte ustar block shape.
fn tar_header(name: &[u8], size: u64) -> [u8; 512] {
    let mut block = [0u8; 512];
    block[..name.len()].copy_from_slice(name);
    let size_field = format!("{size:011o}\0");
    block[124..124 + 12].copy_from_slice(size_field.as_bytes());
    // ustar magic — lets the test prove the bytes round-trip intact.
    block[257..262].copy_from_slice(b"ustar");
    block
}

#[test]
fn tar_shaped_archive_layers_through_gzip() {
    // TAR builder -> gzip encoder -> staged sink, all through Write.
    let payload = b"file body";
    let mut compressed = Vec::new();
    {
        let mut enc =
            gzip::Encoder::new(&mut compressed, gzip::EncodeOptions::default()).expect("encoder");
        enc.write_all(&tar_header(b"entry.txt", payload.len() as u64))
            .expect("tar header");
        enc.write_all(payload).expect("tar body");
        enc.finish().expect("finish");
    }

    // Staged file -> gzip decoder -> TAR parser, all through Read.
    let mut source = BufReader::new(&compressed[..]);
    let mut dec = gzip::Decoder::new(&mut source, gzip::DecodeOptions::default());
    let mut header = [0u8; 512];
    dec.read_exact(&mut header).expect("header block");
    assert_eq!(&header[..9], b"entry.txt");
    assert_eq!(&header[257..262], b"ustar");
    let size = u64::from_str_radix(
        std::str::from_utf8(&header[124..135])
            .unwrap()
            .trim_end_matches('\0'),
        8,
    )
    .unwrap();
    let mut body = vec![0u8; usize::try_from(size).unwrap()];
    dec.read_exact(&mut body).expect("body");
    assert_eq!(body, payload);
}

#[test]
fn io_copy_transcodes_gzip_to_raw_deflate() {
    let payload = b"transcode through std::io only";
    let mut gz = Vec::new();
    {
        let mut enc = gzip::Encoder::new(&mut gz, gzip::EncodeOptions::default()).unwrap();
        enc.write_all(payload).unwrap();
        enc.finish().unwrap();
    }

    let mut raw = Vec::new();
    let mut source = BufReader::new(&gz[..]);
    {
        let mut dec = gzip::Decoder::new(&mut source, gzip::DecodeOptions::default());
        let mut enc = deflate::Encoder::new(&mut raw, deflate::EncodeOptions::default()).unwrap();
        let copied = std::io::copy(&mut dec, &mut enc).expect("transcode");
        assert_eq!(copied, payload.len() as u64);
        enc.finish().expect("deflate finish");
        dec.finish().expect("gzip verified");
    }

    let mut source2 = BufReader::new(&raw[..]);
    let mut dec2 = deflate::Decoder::new(&mut source2, deflate::DecodeOptions::default());
    let mut out = Vec::new();
    dec2.read_to_end(&mut out).expect("decode");
    assert_eq!(out, payload);
}

#[test]
fn adapters_work_through_dyn_trait_objects() {
    // `?Sized` bounds: callers holding only `&mut dyn Write` /
    // `&mut dyn BufRead` can still drive both adapters.
    let mut sink: Vec<u8> = Vec::new();
    let writer: &mut dyn Write = &mut sink;
    let mut enc = gzip::Encoder::new(writer, gzip::EncodeOptions::default()).unwrap();
    enc.write_all(b"via dyn").unwrap();
    enc.finish().unwrap();

    let mut source = BufReader::new(&sink[..]);
    let reader: &mut dyn std::io::BufRead = &mut source;
    let mut dec = gzip::Decoder::new(reader, gzip::DecodeOptions::default());
    let mut out = Vec::new();
    dec.read_to_end(&mut out).unwrap();
    assert_eq!(out, b"via dyn");
    dec.finish().unwrap();
}
