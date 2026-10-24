# Vendored OpenFX headers

Byte-for-byte vendored upstream OpenFX headers for the `ruviform-plugin-ofx`
C bridge and C test fixture (Issue #98).

- Upstream: <https://github.com/AcademySoftwareFoundation/openfx>
- Pin: `c3f1c2dcf4f51762b794c4fc21181b4a501be1bf`
- License: BSD-3-Clause — see `LICENSE.md` (retained in full)
- Integrity: `manifest.json` records sha256 + byte count per file

`ofxInteract.h` is included because `ofxImageEffect.h` references it — the
pinned header closure, not a partial copy.
