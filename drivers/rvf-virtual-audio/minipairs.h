/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    minipairs.h

Abstract:

    Local audio endpoint filter definitions.

--*/

#ifndef _SYSVAD_MINIPAIRS_H_
#define _SYSVAD_MINIPAIRS_H_

// ruviform 派生版 — upstream TabletAudioSample/minipairs.h から
// speaker(render) と mic in(capture) の 1 対のみを残した最小構成。
// 他の endpoint(speakerhp/hdmi/spdif/micarray 等)は登録しない。

#include "speakertopo.h"
#include "speakertoptable.h"
#include "speakerwavtable.h"

#include "micintopo.h"
#include "micintoptable.h"
#include "micinwavtable.h"

NTSTATUS
CreateMiniportWaveRTSYSVAD
(
    _Out_       PUNKNOWN *,
    _In_        REFCLSID,
    _In_opt_    PUNKNOWN,
    _In_        POOL_FLAGS,
    _In_        PUNKNOWN,
    _In_opt_    PVOID,
    _In_        PENDPOINT_MINIPAIR
);

NTSTATUS
CreateMiniportTopologySYSVAD
(
    _Out_       PUNKNOWN *,
    _In_        REFCLSID,
    _In_opt_    PUNKNOWN,
    _In_        POOL_FLAGS,
    _In_        PUNKNOWN,
    _In_opt_    PVOID,
    _In_        PENDPOINT_MINIPAIR
);

//
// Describe buffer size constraints for WaveRT buffers
//
static struct
{
    KSAUDIO_PACKETSIZE_CONSTRAINTS2 TransportPacketConstraints;
    KSAUDIO_PACKETSIZE_PROCESSINGMODE_CONSTRAINT AdditionalProcessingConstraints[1];
} SysvadWaveRtPacketSizeConstraintsRender =
{
    {
        2 * HNSTIME_PER_MILLISECOND,                // 2 ms minimum processing interval
        FILE_BYTE_ALIGNMENT,                        // 1 byte packet size alignment
        0,                                          // no maximum packet size constraint
        2,                                          // 2 processing constraints follow
        {
            STATIC_AUDIO_SIGNALPROCESSINGMODE_DEFAULT,          // constraint for default processing mode
            128,                                                // 128 samples per processing frame
            0,                                                  // NA hns per processing frame
        },
    },
    {
        {
            STATIC_AUDIO_SIGNALPROCESSINGMODE_MOVIE,            // constraint for movie processing mode
            1024,                                               // 1024 samples per processing frame
            0,                                                  // NA hns per processing frame
        },
    }
};

const SYSVAD_DEVPROPERTY SysvadWaveFilterInterfacePropertiesRender[] =
{
    {
        &DEVPKEY_KsAudio_PacketSize_Constraints2,           // Key
        DEVPROP_TYPE_BINARY,                                // Type
        sizeof(SysvadWaveRtPacketSizeConstraintsRender),    // BufferSize
        &SysvadWaveRtPacketSizeConstraintsRender,           // Buffer
    },
};

static struct
{
    KSAUDIO_PACKETSIZE_CONSTRAINTS2 TransportPacketConstraints;
} SysvadWaveRtPacketSizeConstraintsCapture =
{
    {
        2 * HNSTIME_PER_MILLISECOND,                            // 2 ms minimum processing interval
        FILE_BYTE_ALIGNMENT,                                    // 1 byte packet size alignment
        0x100000,                                               // 1 MB maximum packet size
        1,                                                      // 1 processing constraint follows
        {
            STATIC_AUDIO_SIGNALPROCESSINGMODE_COMMUNICATIONS,   // constraint for communications processing mode
            0,                                                  // NA samples per processing frame
            20 * HNSTIME_PER_MILLISECOND,                       // 200000 hns (20ms) per processing frame
        },
    },
};

const SYSVAD_DEVPROPERTY SysvadWaveFilterInterfacePropertiesCapture[] =
{
    {
        &DEVPKEY_KsAudio_PacketSize_Constraints2,           // Key
        DEVPROP_TYPE_BINARY,                                // Type
        sizeof(SysvadWaveRtPacketSizeConstraintsCapture),   // BufferSize
        &SysvadWaveRtPacketSizeConstraintsCapture,          // Buffer
    },
};

//
// Render miniports.
//

/*********************************************************************
* Topology/Wave bridge connection for speaker (internal)             *
*                                                                    *
*              +------+                +------+                      *
*              | Wave |                | Topo |                      *
*              |      |                |      |                      *
* System   --->|0    2|---> Loopback   |      |                      *
*              |      |                |      |                      *
* Offload  --->|1    3|--------------->|0    1|---> Line Out         *
*              |      |                |      |                      *
*              +------+                +------+                      *
*********************************************************************/
static
PHYSICALCONNECTIONTABLE SpeakerTopologyPhysicalConnections[] =
{
    {
        KSPIN_TOPO_WAVEOUT_SOURCE,  // TopologyIn
        KSPIN_WAVE_RENDER_SOURCE,   // WaveOut
        CONNECTIONTYPE_WAVE_OUTPUT
    }
};

static
ENDPOINT_MINIPAIR SpeakerMiniports =
{
    eSpeakerDevice,
    L"TopologySpeaker",                                     // make sure this or the template name matches with KSNAME_TopologySpeaker in the inf's [Strings] section
    NULL,                                                   // optional template name
    CreateMiniportTopologySYSVAD,
    &SpeakerTopoMiniportFilterDescriptor,
    0, NULL,                                                // Interface properties
    L"WaveSpeaker",                                         // make sure this or the template name matches with KSNAME_WaveSpeaker in the inf's [Strings] section
    NULL,                                                   // optional template name
    CreateMiniportWaveRTSYSVAD,
    &SpeakerWaveMiniportFilterDescriptor,
    ARRAYSIZE(SysvadWaveFilterInterfacePropertiesRender),   // Interface properties
    SysvadWaveFilterInterfacePropertiesRender,
    SPEAKER_DEVICE_MAX_CHANNELS,
    SpeakerPinDeviceFormatsAndModes,
    SIZEOF_ARRAY(SpeakerPinDeviceFormatsAndModes),
    SpeakerTopologyPhysicalConnections,
    SIZEOF_ARRAY(SpeakerTopologyPhysicalConnections),
    ENDPOINT_OFFLOAD_SUPPORTED,
    SpeakerModulesWaveFilter,
    SIZEOF_ARRAY(SpeakerModulesWaveFilter),
    &SpeakerModuleNotificationDeviceId,
};

//
// Capture miniports.
//

/*********************************************************************
* Topology/Wave bridge connection for mic in (external)              *
*                                                                    *
*              +------+    +------+                                  *
*              | Topo |    | Wave |                                  *
*              |      |    |      |                                  *
*  Mic in  --->|0    1|===>|0    1|---> Capture Host Pin             *
*              |      |    |      |                                  *
*              +------+    +------+                                  *
*********************************************************************/
static
PHYSICALCONNECTIONTABLE MicInTopologyPhysicalConnections[] =
{
    {
        KSPIN_TOPO_BRIDGE,          // TopologyOut
        KSPIN_WAVE_BRIDGE,          // WaveIn
        CONNECTIONTYPE_TOPOLOGY_OUTPUT
    }
};

static
ENDPOINT_MINIPAIR MicInMiniports =
{
    eMicInDevice,
    L"TopologyMicIn",                       // make sure this or the template name matches with KSNAME_TopologyMicIn in the inf's [Strings] section
    NULL,                                   // optional template name
    CreateMiniportTopologySYSVAD,
    &MicInTopoMiniportFilterDescriptor,
    0, NULL,                                // Interface properties
    L"WaveMicIn",                           // make sure this or the template name matches with KSNAME_WaveMicIn in the inf's [Strings] section
    NULL,                                   // optional template name
    CreateMiniportWaveRTSYSVAD,
    &MicInWaveMiniportFilterDescriptor,
    0, NULL,                                // Interface properties
    MICIN_DEVICE_MAX_CHANNELS,
    MicInPinDeviceFormatsAndModes,
    SIZEOF_ARRAY(MicInPinDeviceFormatsAndModes),
    MicInTopologyPhysicalConnections,
    SIZEOF_ARRAY(MicInTopologyPhysicalConnections),
    ENDPOINT_NO_FLAGS,
    NULL, 0, NULL,                          // audio module settings.
};

//=============================================================================
//
// Render miniport pairs.
//
static
PENDPOINT_MINIPAIR  g_RenderEndpoints[] =
{
    &SpeakerMiniports,
};

#define g_cRenderEndpoints  (SIZEOF_ARRAY(g_RenderEndpoints))

//=============================================================================
//
// Capture miniport pairs.
//
static
PENDPOINT_MINIPAIR  g_CaptureEndpoints[] =
{
    &MicInMiniports,
};

#define g_cCaptureEndpoints (SIZEOF_ARRAY(g_CaptureEndpoints))

//=============================================================================
//
// Total miniports = # endpoints * 2 (topology + wave).
//
#define g_MaxMiniports  ((g_cRenderEndpoints + g_cCaptureEndpoints) * 2)

//=============================================================================
//
// ruviform: MIDI loopback (DMus) filters — endpoint pair 系統とは独立した
// 単独 filter。RvfMidi.h の CreateMiniportRvfMidi が miniport factory。
//
#define g_MaxMidiFilters  (1)

#endif // _SYSVAD_MINIPAIRS_H_
