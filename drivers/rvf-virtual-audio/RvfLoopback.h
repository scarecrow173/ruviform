/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    RvfLoopback.h

Abstract:

    Render-to-capture loopback ("virtual cable") data path.

    SYSVAD has no cross-endpoint data path: each pin synthesizes or drops
    data locally. RvfVirtualAudio adds a single loopback source that the
    capture stream reads when filling its WaveRT buffer so PCM rendered to
    the speaker endpoint ("Ruviform Cable Input") is observable on the
    microphone endpoint ("Ruviform Cable Output").

    All conversion math below is integer-only so it can run at
    DISPATCH_LEVEL (the capture fill is invoked from the WaveRT position
    notification DPC). No floating point state is touched.

--*/

#ifndef _SYSVAD_RVFLOOPBACK_H_
#define _SYSVAD_RVFLOOPBACK_H_

//=============================================================================
// RVF_LOOPBACK_FORMAT
//
// Normalized sample format descriptor. Extracted once from the negotiated
// WAVEFORMATEX at registration time so the hot copy loop never parses
// WAVEFORMATEXTENSIBLE.
//=============================================================================
typedef struct _RVF_LOOPBACK_FORMAT
{
    USHORT      Channels;               // channel count
    USHORT      BitsPerSample;          // container bits per sample
    ULONG       SamplesPerSec;          // frame rate
    ULONG       BlockAlign;             // bytes per frame
    BOOLEAN     IsFloat;                // IEEE 32-bit float samples
    BOOLEAN     Valid;                  // format usable for conversion
} RVF_LOOPBACK_FORMAT, *PRVF_LOOPBACK_FORMAT;

//=============================================================================
// RVF_LOOPBACK_RENDER_SOURCE
//
// Shared view of the render (speaker) stream's WaveRT DMA buffer. Owned by
// CAdapterCommon; the render stream registers/unregisters it across the
// DMA buffer lifetime, publishes its play cursor and RUN state, and the
// capture stream copies out of it under the adapter loopback spinlock.
//=============================================================================
typedef struct _RVF_LOOPBACK_RENDER_SOURCE
{
    PVOID                   Owner;          // registering stream instance
    BYTE*                   DmaBuffer;      // mapped render WaveRT buffer
    ULONG                   DmaBufferSize;  // buffer size in bytes
    RVF_LOOPBACK_FORMAT     Format;         // negotiated render pin format
    volatile ULONGLONG      PlayBytePos;    // linear bytes played (cursor)
    ULONGLONG               ReadBytePos;    // capture-side contiguous read cursor
    BOOLEAN                 ReadPosValid;   // ReadBytePos was anchored
    BOOLEAN                 Running;        // stream is in KSSTATE_RUN
    BOOLEAN                 Registered;     // registration is live
} RVF_LOOPBACK_RENDER_SOURCE, *PRVF_LOOPBACK_RENDER_SOURCE;

//=============================================================================
// RvfLoopbackFormatFromWfx
//
// Extracts a normalized RVF_LOOPBACK_FORMAT from a negotiated WAVEFORMATEX
// (WAVEFORMATEXTENSIBLE-typed pointer is fine; only the WAVEFORMATEX fields
// plus SubFormat are inspected). Marks Valid=FALSE for formats the copy
// path cannot handle.
//=============================================================================
VOID
RvfLoopbackFormatFromWfx
(
    _In_        const WAVEFORMATEX      *Wfx,
    _Out_       PRVF_LOOPBACK_FORMAT    Format
);

//=============================================================================
// RvfLoopbackCopyToCapture
//
// Fills Dst (a capture WaveRT buffer span) with audio read from the render
// buffer through a contiguous read cursor (ReadBytePos): the cursor is
// anchored once to the live play position and then advances by exactly the
// consumed source frames, so consecutive fills never duplicate or drop
// content. The buffer is circular, so when the capture falls behind the
// live cursor it reads older (still valid) stimulus audio instead of
// skipping. Tearing against concurrent app writes is acceptable: the
// loopback is an observation path, not a bit-exact pipe.
//
// When the source is not registered, not running, or the formats are not
// usable, the destination is filled with silence (virtual-cable behavior).
//
// IRQL: <= DISPATCH_LEVEL. Caller serializes access to Source (the
// function updates Source->ReadBytePos/ReadPosValid).
//=============================================================================
VOID
RvfLoopbackCopyToCapture
(
    _Inout_                         PRVF_LOOPBACK_RENDER_SOURCE   Source,
    _Out_writes_bytes_(DstByteCount)    BYTE                     *Dst,
    _In_                                ULONG                     DstByteCount,
    _In_                            const RVF_LOOPBACK_FORMAT    *DstFormat
);

#endif  // _SYSVAD_RVFLOOPBACK_H_
