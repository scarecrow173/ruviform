/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    RvfLoopback.cpp

Abstract:

    Render-to-capture loopback ("virtual cable") copy/convert path for
    RvfVirtualAudio. See RvfLoopback.h for the data-path description.

    Integer math only: this code is invoked under a spinlock at
    DISPATCH_LEVEL from the capture stream's position update, so no
    floating point or KeSaveFloatingPointState is used anywhere below.

--*/

#include <sysvad.h>
#include "RvfLoopback.h"

//=============================================================================
// Sample decode/encode helpers.
//
// All samples pass through a signed 32-bit normalized domain: a value of
// 0x7FFFFFFF is +full-scale and -0x80000000 is -full-scale regardless of
// the wire bit depth or encoding.
//=============================================================================

//
// Decode a signed PCM little-endian sample (16/24/32-bit) to normalized S32.
//
static
LONGLONG
RvfReadPcmS32
(
    _In_    const BYTE  *Sample,
    _In_    ULONG        BytesPerSample
)
{
    switch (BytesPerSample)
    {
    case 2:
        {
            SHORT v;
            RtlCopyMemory(&v, Sample, sizeof(v));
            return ((LONGLONG)v) << 16;
        }
    case 3:
        {
            LONG v = (LONG)((ULONG)Sample[0] |
                           ((ULONG)Sample[1] << 8) |
                           ((ULONG)Sample[2] << 16));
            if (v & 0x800000)
            {
                v |= ~0xFFFFFFL;
            }
            return ((LONGLONG)v) << 8;
        }
    case 4:
        {
            LONG v;
            RtlCopyMemory(&v, Sample, sizeof(v));
            return (LONGLONG)v;
        }
    default:
        return 0;
    }
}

//
// Decode an IEEE float32 sample to normalized S32 using pure integer math
// on the bit fields (no FPU at DISPATCH_LEVEL).
//
static
LONGLONG
RvfReadF32S32
(
    _In_    const BYTE  *Sample
)
{
    ULONG bits;
    ULONG sign;
    ULONG exponent;
    ULONGLONG mantissa;
    LONGLONG v;
    int shift;

    RtlCopyMemory(&bits, Sample, sizeof(bits));

    sign      = bits & 0x80000000UL;
    exponent  = (bits >> 23) & 0xFF;
    mantissa  = bits & 0x7FFFFFULL;

    if (exponent == 0)
    {
        // Zero or denormal: too small to matter at S32 scale.
        return 0;
    }

    if (exponent == 0xFF)
    {
        // Inf/NaN: saturate to full scale with the original sign.
        return sign ? (LONGLONG)(-0x80000000LL) : (LONGLONG)0x7FFFFFFFLL;
    }

    mantissa |= 0x800000ULL;

    //
    // f = mantissa * 2^(exponent - 150). We want v ~= f * 2^31,
    // so v = mantissa * 2^(exponent - 119). mantissa is in [2^23, 2^24),
    // so a left shift of >= 8 overflows S32 and a right shift of >= 24
    // underflows to zero; both bounds are explicit to keep the shifts
    // well-defined.
    //
    shift = (int)exponent - 119;
    if (shift >= 8)
    {
        v = 0x7FFFFFFFLL;
    }
    else if (shift >= 0)
    {
        v = (LONGLONG)(mantissa << shift);
    }
    else if (shift <= -24)
    {
        v = 0;
    }
    else
    {
        v = (LONGLONG)(mantissa >> (-shift));
    }

    return sign ? -v : v;
}

//
// Decode one sample of the given format to normalized S32.
//
static
LONGLONG
RvfDecodeSample
(
    _In_    const BYTE                  *Sample,
    _In_    const RVF_LOOPBACK_FORMAT   *Format
)
{
    if (Format->IsFloat)
    {
        return (Format->BitsPerSample == 32) ? RvfReadF32S32(Sample) : 0;
    }
    return RvfReadPcmS32(Sample, Format->BitsPerSample / 8);
}

//
// Encode normalized S32 as a signed PCM little-endian sample (16/24/32-bit).
//
static
VOID
RvfWritePcmS32
(
    _Out_   BYTE    *Sample,
    _In_    ULONG    BytesPerSample,
    _In_    LONGLONG Value
)
{
    switch (BytesPerSample)
    {
    case 2:
        {
            LONG q = (LONG)(Value >> 16);
            SHORT v;
            if (q > 32767)      q = 32767;
            if (q < -32768)     q = -32768;
            v = (SHORT)q;
            RtlCopyMemory(Sample, &v, sizeof(v));
            break;
        }
    case 3:
        {
            LONG v = (LONG)(Value >> 8);
            if (v > 8388607)    v = 8388607;
            if (v < -8388608)   v = -8388608;
            Sample[0] = (BYTE)(v & 0xFF);
            Sample[1] = (BYTE)((v >> 8) & 0xFF);
            Sample[2] = (BYTE)((v >> 16) & 0xFF);
            break;
        }
    case 4:
        {
            LONG64 q = Value;
            LONG v;
            if (q > 0x7FFFFFFFLL)   q = 0x7FFFFFFFLL;
            if (q < (-0x80000000LL)) q = -0x80000000LL;
            v = (LONG)q;
            RtlCopyMemory(Sample, &v, sizeof(v));
            break;
        }
    default:
        break;
    }
}

//
// Encode normalized S32 as IEEE float32 using pure integer math.
//
static
VOID
RvfWriteF32S32
(
    _Out_   BYTE    *Sample,
    _In_    LONGLONG Value
)
{
    ULONG bits;
    ULONG sign = 0;
    ULONGLONG a;
    int top;
    ULONG exponent;
    ULONGLONG mantissa;

    if (Value == 0)
    {
        bits = 0;
        RtlCopyMemory(Sample, &bits, sizeof(bits));
        return;
    }

    if (Value < 0)
    {
        sign = 0x80000000UL;
        // Value >= -2^31 in this domain, so -Value is representable.
        a = (ULONGLONG)(-Value);
    }
    else
    {
        a = (ULONGLONG)Value;
    }

    //
    // a is in [1, 2^31]. f = a / 2^31 in (0, 1]. Find the index of the
    // highest set bit (top <= 31); bounded loop, safe at DISPATCH_LEVEL.
    //
    top = 31;
    while (!(a & (1ULL << top)))
    {
        --top;
    }

    // f = a * 2^-31 = M * 2^(E-150) with M in [2^23, 2^24)
    //   => E = top + 96, M = a * 2^(23 - top)
    exponent = (ULONG)(top + 96);
    if (top >= 23)
    {
        mantissa = (a >> (top - 23)) & 0x7FFFFFULL;
    }
    else
    {
        mantissa = (a << (23 - top)) & 0x7FFFFFULL;
    }

    bits = sign | (exponent << 23) | (ULONG)mantissa;
    RtlCopyMemory(Sample, &bits, sizeof(bits));
}

//
// Encode one normalized S32 sample in the given format.
//
static
VOID
RvfEncodeSample
(
    _Out_   BYTE                        *Sample,
    _In_    LONGLONG                     Value,
    _In_    const RVF_LOOPBACK_FORMAT   *Format
)
{
    if (Format->IsFloat)
    {
        if (Format->BitsPerSample == 32)
        {
            RvfWriteF32S32(Sample, Value);
        }
        return;
    }
    RvfWritePcmS32(Sample, Format->BitsPerSample / 8, Value);
}

//=============================================================================
// RvfLoopbackFormatFromWfx
//=============================================================================
#pragma code_seg()
VOID
RvfLoopbackFormatFromWfx
(
    _In_        const WAVEFORMATEX      *Wfx,
    _Out_       PRVF_LOOPBACK_FORMAT    Format
)
{
    RtlZeroMemory(Format, sizeof(*Format));

    if (Wfx == NULL || Wfx->nChannels == 0 || Wfx->nBlockAlign == 0 ||
        Wfx->nSamplesPerSec == 0)
    {
        return;
    }

    Format->Channels      = Wfx->nChannels;
    Format->BitsPerSample = Wfx->wBitsPerSample;
    Format->SamplesPerSec = Wfx->nSamplesPerSec;
    Format->BlockAlign    = Wfx->nBlockAlign;

    if (Wfx->wFormatTag == WAVE_FORMAT_IEEE_FLOAT)
    {
        Format->IsFloat = TRUE;
    }
    else if (Wfx->wFormatTag == WAVE_FORMAT_EXTENSIBLE)
    {
        const WAVEFORMATEXTENSIBLE *wfExt =
            CONTAINING_RECORD(Wfx, WAVEFORMATEXTENSIBLE, Format);
        Format->IsFloat =
            IsEqualGUIDAligned(wfExt->SubFormat, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT)
                ? TRUE : FALSE;
    }
    else if (Wfx->wFormatTag != WAVE_FORMAT_PCM &&
             Wfx->wFormatTag != WAVE_FORMAT_EXTENSIBLE)
    {
        // Non-PCM, non-float encodings are not converted.
        return;
    }

    // Only 16/24/32-bit PCM or 32-bit float are supported.
    if ((!Format->IsFloat &&
         !(Format->BitsPerSample == 16 || Format->BitsPerSample == 24 ||
           Format->BitsPerSample == 32)) ||
        (Format->IsFloat && Format->BitsPerSample != 32))
    {
        return;
    }

    Format->Valid = TRUE;
}

//=============================================================================
// RvfLoopbackCopyToCapture
//=============================================================================
#pragma code_seg()
VOID
RvfLoopbackCopyToCapture
(
    _Inout_                         PRVF_LOOPBACK_RENDER_SOURCE   Source,
    _Out_writes_bytes_(DstByteCount)    BYTE                     *Dst,
    _In_                                ULONG                     DstByteCount,
    _In_                            const RVF_LOOPBACK_FORMAT    *DstFormat
)
{
    ULONGLONG   srcFrameCursor;
    ULONG       srcBytesPerSample;
    ULONG       dstBytesPerSample;
    ULONGLONG   dstFrames;

    //
    // No live render source, or a format we cannot convert: a virtual
    // cable with nothing playing carries silence.
    //
    if (!Source->Registered || !Source->Running || Source->DmaBuffer == NULL ||
        Source->DmaBufferSize == 0 || !Source->Format.Valid ||
        Source->Format.BlockAlign == 0 ||
        DstFormat == NULL || !DstFormat->Valid || DstFormat->BlockAlign == 0)
    {
        RtlZeroMemory(Dst, DstByteCount);
        return;
    }

    srcBytesPerSample = Source->Format.BlockAlign / Source->Format.Channels;
    dstBytesPerSample = DstFormat->BlockAlign / DstFormat->Channels;
    dstFrames         = DstByteCount / DstFormat->BlockAlign;

    //
    // Contiguous read cursor: advance by exactly the consumed frames so
    // consecutive fills neither duplicate nor drop content — re-anchoring
    // on every fill let the two streams' independent position clocks
    // jitter the boundary and smeared the captured spectrum. Re-anchor
    // only when the cursor has drifted implausibly far from live (half
    // the circular buffer): inside that band the read region still holds
    // recently written stimulus, so continuity beats chasing the cursor.
    //
    {
        LONGLONG drift = (LONGLONG)(Source->PlayBytePos - Source->ReadBytePos);

        if (!Source->ReadPosValid ||
            drift > (LONGLONG)(Source->DmaBufferSize / 2) ||
            drift < -(LONGLONG)(Source->DmaBufferSize / 2))
        {
            Source->ReadBytePos  = Source->PlayBytePos;
            Source->ReadPosValid = TRUE;
        }
    }

    srcFrameCursor = Source->ReadBytePos / Source->Format.BlockAlign;

    for (ULONGLONG dstFrame = 0; dstFrame < dstFrames; ++dstFrame)
    {
        BYTE *dstFramePtr = Dst + (ULONG)(dstFrame * DstFormat->BlockAlign);
        ULONGLONG srcFrameIndex;
        ULONGLONG srcByteOffset;
        const BYTE *srcFramePtr;

        //
        // Nearest-neighbor rate mapping; identical when rates match (the
        // common 48 kHz <-> 48 kHz case degenerates to a straight copy).
        //
        srcFrameIndex = srcFrameCursor +
            (dstFrame * Source->Format.SamplesPerSec) / DstFormat->SamplesPerSec;
        srcByteOffset = (srcFrameIndex * Source->Format.BlockAlign) %
                        Source->DmaBufferSize;
        srcFramePtr   = Source->DmaBuffer + (ULONG)srcByteOffset;

        for (ULONG dstCh = 0; dstCh < DstFormat->Channels; ++dstCh)
        {
            LONGLONG value;

            if (DstFormat->Channels == 1 && Source->Format.Channels > 1)
            {
                // Downmix: arithmetic mean of all source channels.
                LONGLONG sum = 0;
                for (ULONG srcCh = 0; srcCh < Source->Format.Channels; ++srcCh)
                {
                    sum += RvfDecodeSample(
                        srcFramePtr + srcCh * srcBytesPerSample,
                        &Source->Format);
                }
                value = sum / Source->Format.Channels;
            }
            else
            {
                // Channel-mapped copy: dst channel j takes src channel j,
                // replicating source channel 0 when the destination has
                // more channels than the source.
                ULONG srcCh =
                    (dstCh < Source->Format.Channels) ? dstCh : 0;
                value = RvfDecodeSample(
                    srcFramePtr + srcCh * srcBytesPerSample,
                    &Source->Format);
            }

            RvfEncodeSample(dstFramePtr + dstCh * dstBytesPerSample,
                            value, DstFormat);
        }
    }

    //
    // Advance the read cursor by the consumed source frames — matching
    // the per-frame index mapping so the next fill continues seamlessly.
    //
    Source->ReadBytePos =
        (srcFrameCursor +
         (dstFrames * Source->Format.SamplesPerSec) / DstFormat->SamplesPerSec) *
        Source->Format.BlockAlign;

    //
    // Trailing bytes that do not form a whole destination frame.
    //
    {
        ULONG tail = DstByteCount % DstFormat->BlockAlign;
        if (tail != 0)
        {
            RtlZeroMemory(Dst + DstByteCount - tail, tail);
        }
    }
}
