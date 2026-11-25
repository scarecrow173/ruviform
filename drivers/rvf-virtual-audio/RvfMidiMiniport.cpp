/*****************************************************************************
 * RvfMidiMiniport.cpp - Ruviform virtual MIDI loopback miniport implementation
 *****************************************************************************
 * Copyright (c) 1997-2000 Microsoft Corporation.  All Rights Reserved.
 *
 * ruviform 派生版 — WDK 8.1 "DirectMusic UART Driver Sample" (DMusUART) の
 * miniport.cpp 由来。MPU-401 ハードウェア UART I/O を software
 * render→capture loopback に置き換えた（出所・変更点は PROVENANCE.md 参照）。
 *
 * ハードウェア非依存化の要点:
 *  - root-enumerated device のため IRQ/I/O port resource を一切使わない
 *    （InterruptSync は生成しない — Notify/ServiceGroup 経路のみで
 *    capture を駆動する）。
 *  - render stream の Write() が byte 列を内部 FIFO へ積み、
 *    IPortDMus::Notify で capture stream の SourceEvtsToPort を駆動する
 *    （実 UART への write → ISR → FIFO の経路をソフトウェア loopback に
 *    置き換え。byte 経路は RvfMidiMpu.cpp）。
 *  - pin-name GUID は render/capture 両 pin で同一の独自 GUID
 *    （RVF_MIDI_LOOPBACK_NAME）を共有し、INF の MediaCategories 登録で
 *    "Ruviform MIDI Loopback" 文字列に結び付ける — WinMM midiIn/midiOut
 *    で同名ポートを列挙させることが loopback.rs の前提。
 */

#pragma warning (disable : 4127)

#include "RvfMidi.h"
#include "ksdebug.h"

#define STR_MODULENAME "RvfMidi:Miniport: "

// {714b96c3-8003-41c2-b673-1b086cbdb3db}
// pin-name GUID の唯一の定義 — initguid 機構を通さずこの TU で管理する
// （RvfMidi.h のコメント参照）。render/capture 両 pin が同一値を指し、
// INF の MediaCategories 登録が "Ruviform MIDI Loopback" に結び付ける。
EXTERN_C const GUID FAR RVF_MIDI_LOOPBACK_NAME =
    {0x714b96c3, 0x8003, 0x41c2, {0xb6, 0x73, 0x1b, 0x08, 0x6c, 0xbd, 0xb3}};

#pragma code_seg("PAGE")

/*****************************************************************************
 * PinDataRangesStreamLegacy
 * PinDataRangesStreamDMusic
 *****************************************************************************
 * Structures indicating range of valid format values for live pins.
 */
static
KSDATARANGE_MUSIC PinDataRangesStreamLegacy =
{
    {
        sizeof(KSDATARANGE_MUSIC),
        0,
        0,
        0,
        STATICGUIDOF(KSDATAFORMAT_TYPE_MUSIC),
        STATICGUIDOF(KSDATAFORMAT_SUBTYPE_MIDI),
        STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE)
    },
    STATICGUIDOF(KSMUSIC_TECHNOLOGY_PORT),
    0,
    0,
    0xFFFF
};
static
KSDATARANGE_MUSIC PinDataRangesStreamDMusic =
{
    {
        sizeof(KSDATARANGE_MUSIC),
        0,
        0,
        0,
        STATICGUIDOF(KSDATAFORMAT_TYPE_MUSIC),
        STATICGUIDOF(KSDATAFORMAT_SUBTYPE_DIRECTMUSIC),
        STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE)
    },
    STATICGUIDOF(KSMUSIC_TECHNOLOGY_PORT),
    0,
    0,
    0xFFFF
};

/*****************************************************************************
 * PinDataRangePointersStreamLegacy
 * PinDataRangePointersStreamDMusic
 * PinDataRangePointersStreamCombined
 *****************************************************************************
 * List of pointers to structures indicating range of valid format values
 * for live pins.
 */
static
PKSDATARANGE PinDataRangePointersStreamLegacy[] =
{
    PKSDATARANGE(&PinDataRangesStreamLegacy)
};
static
PKSDATARANGE PinDataRangePointersStreamDMusic[] =
{
    PKSDATARANGE(&PinDataRangesStreamDMusic)
};
static
PKSDATARANGE PinDataRangePointersStreamCombined[] =
{
    PKSDATARANGE(&PinDataRangesStreamLegacy)
   ,PKSDATARANGE(&PinDataRangesStreamDMusic)
};

/*****************************************************************************
 * PinDataRangesBridge
 *****************************************************************************
 * Structures indicating range of valid format values for bridge pins.
 */
static
KSDATARANGE PinDataRangesBridge[] =
{
   {
      sizeof(KSDATARANGE),
      0,
      0,
      0,
      STATICGUIDOF(KSDATAFORMAT_TYPE_MUSIC),
      STATICGUIDOF(KSDATAFORMAT_SUBTYPE_MIDI_BUS),
      STATICGUIDOF(KSDATAFORMAT_SPECIFIER_NONE)
   }
};

/*****************************************************************************
 * PinDataRangePointersBridge
 *****************************************************************************
 * List of pointers to structures indicating range of valid format values
 * for bridge pins.
 */
static
PKSDATARANGE PinDataRangePointersBridge[] =
{
    &PinDataRangesBridge[0]
};

/*****************************************************************************
 * SynthProperties
 *****************************************************************************
 * List of properties in the Synth set.
 */
static
PCPROPERTY_ITEM
SynthProperties[] =
{
    // Global: S/Get synthesizer caps
    {
        &KSPROPSETID_Synth,
        KSPROPERTY_SYNTH_CAPS,
        KSPROPERTY_TYPE_GET | KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_Synth
    },
    // Global: S/Get port parameters
    {
        &KSPROPSETID_Synth,
        KSPROPERTY_SYNTH_PORTPARAMETERS,
        KSPROPERTY_TYPE_GET | KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_Synth
    },
    // Per stream: S/Get channel groups
    {
        &KSPROPSETID_Synth,
        KSPROPERTY_SYNTH_CHANNELGROUPS,
        KSPROPERTY_TYPE_SET | KSPROPERTY_TYPE_GET | KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_Synth
    },
    // Per stream: Get current latency time
    {
        &KSPROPSETID_Synth,
        KSPROPERTY_SYNTH_LATENCYCLOCK,
        KSPROPERTY_TYPE_GET | KSPROPERTY_TYPE_BASICSUPPORT,
        PropertyHandler_Synth
    }
};
DEFINE_PCAUTOMATION_TABLE_PROP(AutomationSynth,  SynthProperties);
DEFINE_PCAUTOMATION_TABLE_PROP(AutomationSynth2, SynthProperties);

#define kMaxNumCaptureStreams       1
#define kMaxNumLegacyRenderStreams  1
#define kMaxNumDMusicRenderStreams  1

/*****************************************************************************
 * MiniportPins
 *****************************************************************************
 * List of pins.
 */
static
PCPIN_DESCRIPTOR MiniportPins[] =
{
    {
        kMaxNumLegacyRenderStreams,kMaxNumLegacyRenderStreams,0,    // InstanceCount
        NULL,                                                       // AutomationTable
        {                                                           // KsPinDescriptor
            0,                                              // InterfacesCount
            NULL,                                           // Interfaces
            0,                                              // MediumsCount
            NULL,                                           // Mediums
            SIZEOF_ARRAY(PinDataRangePointersStreamLegacy), // DataRangesCount
            PinDataRangePointersStreamLegacy,               // DataRanges
            KSPIN_DATAFLOW_IN,                              // DataFlow
            KSPIN_COMMUNICATION_SINK,                       // Communication
            (GUID *) &KSCATEGORY_AUDIO,                     // Category
            &RVF_MIDI_LOOPBACK_NAME,                        // Name
            0                                               // Reserved
        }
    },
    {
        kMaxNumDMusicRenderStreams,kMaxNumDMusicRenderStreams,0,    // InstanceCount
        NULL,                                                       // AutomationTable
        {                                                           // KsPinDescriptor
            0,                                              // InterfacesCount
            NULL,                                           // Interfaces
            0,                                              // MediumsCount
            NULL,                                           // Mediums
            SIZEOF_ARRAY(PinDataRangePointersStreamDMusic), // DataRangesCount
            PinDataRangePointersStreamDMusic,               // DataRanges
            KSPIN_DATAFLOW_IN,                              // DataFlow
            KSPIN_COMMUNICATION_SINK,                       // Communication
            (GUID *) &KSCATEGORY_AUDIO,                     // Category
            &RVF_MIDI_LOOPBACK_NAME,                        // Name
            0                                               // Reserved
        }
    },
    {
        0,0,0,                                      // InstanceCount
        NULL,                                       // AutomationTable
        {                                           // KsPinDescriptor
            0,                                          // InterfacesCount
            NULL,                                       // Interfaces
            0,                                          // MediumsCount
            NULL,                                       // Mediums
            SIZEOF_ARRAY(PinDataRangePointersBridge),   // DataRangesCount
            PinDataRangePointersBridge,                 // DataRanges
            KSPIN_DATAFLOW_OUT,                         // DataFlow
            KSPIN_COMMUNICATION_NONE,                   // Communication
            (GUID *) &KSCATEGORY_AUDIO,                 // Category
            NULL,                                       // Name
            0                                           // Reserved
        }
    },
    {
        0,0,0,                                      // InstanceCount
        NULL,                                       // AutomationTable
        {                                           // KsPinDescriptor
            0,                                          // InterfacesCount
            NULL,                                       // Interfaces
            0,                                          // MediumsCount
            NULL,                                       // Mediums
            SIZEOF_ARRAY(PinDataRangePointersBridge),   // DataRangesCount
            PinDataRangePointersBridge,                 // DataRanges
            KSPIN_DATAFLOW_IN,                          // DataFlow
            KSPIN_COMMUNICATION_NONE,                   // Communication
            (GUID *) &KSCATEGORY_AUDIO,                 // Category
            NULL,                                       // Name
            0                                           // Reserved
        }
    },
    {
        kMaxNumCaptureStreams,kMaxNumCaptureStreams,0,      // InstanceCount
        NULL,                                               // AutomationTable
        {                                                   // KsPinDescriptor
            0,                                                // InterfacesCount
            NULL,                                             // Interfaces
            0,                                                // MediumsCount
            NULL,                                             // Mediums
            SIZEOF_ARRAY(PinDataRangePointersStreamCombined), // DataRangesCount
            PinDataRangePointersStreamCombined,               // DataRanges
            KSPIN_DATAFLOW_OUT,                               // DataFlow
            KSPIN_COMMUNICATION_SINK,                         // Communication
            (GUID *) &KSCATEGORY_AUDIO,                       // Category
            &RVF_MIDI_LOOPBACK_NAME,                          // Name
            0                                                 // Reserved
        }
    }
};

/*****************************************************************************
 * MiniportNodes
 *****************************************************************************
 * List of nodes.
 */
#define CONST_PCNODE_DESCRIPTOR(n)          { 0, NULL, &n, NULL }
#define CONST_PCNODE_DESCRIPTOR_AUTO(n,a)   { 0, &a, &n, NULL }
static
PCNODE_DESCRIPTOR MiniportNodes[] =
{
      CONST_PCNODE_DESCRIPTOR_AUTO(KSNODETYPE_SYNTHESIZER, AutomationSynth)
    , CONST_PCNODE_DESCRIPTOR_AUTO(KSNODETYPE_SYNTHESIZER, AutomationSynth2)
};

/*****************************************************************************
 * MiniportConnections
 *****************************************************************************
 * List of connections.
 */
enum {
      eSynthNode  = 0
    , eInputNode
};

enum {
      eFilterInputPinLeg = 0,
      eFilterInputPinDM,
      eBridgeOutputPin,
      eBridgeInputPin,
      eFilterOutputPin
};

static
PCCONNECTION_DESCRIPTOR MiniportConnections[] =
{  // From                                   To
   // Node           pin                     Node           pin
    { PCFILTER_NODE, eFilterInputPinLeg,     PCFILTER_NODE, eBridgeOutputPin }      // Legacy Stream in to synth.
  , { PCFILTER_NODE, eFilterInputPinDM,      eSynthNode,    KSNODEPIN_STANDARD_IN } // DM Stream in to synth.
  , { eSynthNode,    KSNODEPIN_STANDARD_OUT, PCFILTER_NODE, eBridgeOutputPin }      // Synth to bridge out.
  , { PCFILTER_NODE, eBridgeInputPin,        eInputNode,    KSNODEPIN_STANDARD_IN } // Bridge in to input.
  , { eInputNode,    KSNODEPIN_STANDARD_OUT, PCFILTER_NODE, eFilterOutputPin }      // Input to DM/Legacy Stream out.
};

/*****************************************************************************
 * MiniportCategories
 *****************************************************************************
 * List of categories.
 */
static
GUID MiniportCategories[] =
{
    STATICGUIDOF(KSCATEGORY_AUDIO),
    STATICGUIDOF(KSCATEGORY_RENDER),
    STATICGUIDOF(KSCATEGORY_CAPTURE)
};

/*****************************************************************************
 * MiniportFilterDescriptor
 *****************************************************************************
 * Complete miniport filter description.
 */
static
PCFILTER_DESCRIPTOR MiniportFilterDescriptor =
{
    0,                                  // Version
    NULL,                               // AutomationTable
    sizeof(PCPIN_DESCRIPTOR),           // PinSize
    SIZEOF_ARRAY(MiniportPins),         // PinCount
    MiniportPins,                       // Pins
    sizeof(PCNODE_DESCRIPTOR),          // NodeSize
    SIZEOF_ARRAY(MiniportNodes),        // NodeCount
    MiniportNodes,                      // Nodes
    SIZEOF_ARRAY(MiniportConnections),  // ConnectionCount
    MiniportConnections,                // Connections
    SIZEOF_ARRAY(MiniportCategories),   // CategoryCount
    MiniportCategories                  // Categories
};

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidi::GetDescription()
 *****************************************************************************
 * Gets the topology.
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidi::
GetDescription
(
    OUT     PPCFILTER_DESCRIPTOR *  OutFilterDescriptor
)
{
    PAGED_CODE();

    ASSERT(OutFilterDescriptor);

    _DbgPrintF(DEBUGLVL_BLAB,("GetDescription"));

    *OutFilterDescriptor = &MiniportFilterDescriptor;

    return STATUS_SUCCESS;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CreateMiniportRvfMidi()
 *****************************************************************************
 * Creates the virtual MIDI loopback miniport driver for the adapter.
 * PFNCREATEMINIPORT 形 — CAdapterCommon::InstallSubdevice から呼ばれる。
 * ClassId/UnknownAdapter/DeviceContext/MiniportPair は DMus port 固有
 * では使用しない。
 */
NTSTATUS
CreateMiniportRvfMidi
(
    _Out_       PUNKNOWN *          Unknown,
    _In_        REFCLSID            ClassId,
    _In_opt_    PUNKNOWN            UnknownOuter,
    _In_        POOL_FLAGS          PoolFlags,
    _In_        PUNKNOWN            UnknownAdapter,
    _In_opt_    PVOID               DeviceContext,
    _In_        PENDPOINT_MINIPAIR  MiniportPair
)
{
    PAGED_CODE();

    UNREFERENCED_PARAMETER(ClassId);
    UNREFERENCED_PARAMETER(UnknownAdapter);
    UNREFERENCED_PARAMETER(DeviceContext);
    UNREFERENCED_PARAMETER(MiniportPair);

    _DbgPrintF(DEBUGLVL_BLAB, ("CreateMiniportRvfMidi"));
    ASSERT(Unknown);

    *Unknown = NULL;

    CMiniportRvfMidi *pMiniport =
        new(PoolFlags, MINMIDI_POOLTAG) CMiniportRvfMidi(UnknownOuter);

    if (pMiniport)
    {
        *Unknown = PUNKNOWN(PMINIPORTDMUS(pMiniport));
        (*Unknown)->AddRef();
        return STATUS_SUCCESS;
    }

    return STATUS_INSUFFICIENT_RESOURCES;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidi::NonDelegatingQueryInterface()
 *****************************************************************************
 * Obtains an interface.  This function works just like a COM QueryInterface
 * call and is used if the object is not being aggregated.
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidi::
NonDelegatingQueryInterface
(
    _In_           REFIID  Interface,
    _COM_Outptr_   PVOID * Object
)
{
    PAGED_CODE();

    _DbgPrintF(DEBUGLVL_BLAB, ("Miniport::NonDelegatingQueryInterface"));
    ASSERT(Object);

    if (IsEqualGUIDAligned(Interface,IID_IUnknown))
    {
        *Object = PVOID(PUNKNOWN(PMINIPORTDMUS(this)));
    }
    else
    if (IsEqualGUIDAligned(Interface,IID_IMiniport))
    {
        *Object = PVOID(PMINIPORT(this));
    }
    else
    if (IsEqualGUIDAligned(Interface,IID_IMiniportDMus))
    {
        *Object = PVOID(PMINIPORTDMUS(this));
    }
    else
    if (IsEqualGUIDAligned(Interface,IID_IMusicTechnology))
    {
        *Object = PVOID(PMUSICTECHNOLOGY(this));
    }
    else
    if (IsEqualGUIDAligned(Interface,IID_IPowerNotify))
    {
        *Object = PVOID(PPOWERNOTIFY(this));
    }
    else
    {
        *Object = NULL;
    }

    if (*Object)
    {
        //
        // We reference the interface for the caller.
        //
        PUNKNOWN(*Object)->AddRef();
        return STATUS_SUCCESS;
    }

    return STATUS_INVALID_PARAMETER;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidi::~CMiniportRvfMidi()
 *****************************************************************************
 * Destructor.
 */
CMiniportRvfMidi::~CMiniportRvfMidi(void)
{
    PAGED_CODE();

    _DbgPrintF(DEBUGLVL_BLAB,("~CMiniportRvfMidi"));

    ASSERT(0 == m_NumCaptureStreams);
    ASSERT(0 == m_NumRenderStreams);

    if (m_pServiceGroup)
    {
        m_pServiceGroup->Release();
        m_pServiceGroup = NULL;
    }
    if (m_pPort)
    {
        m_pPort->Release();
        m_pPort = NULL;
    }
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidi::Init()
 *****************************************************************************
 * Initializes the miniport.  Virtual device のため resource list は参照
 * しない（IRQ/I/O port 不使用 — InterruptSync も生成しない）。
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidi::
Init
(
    IN      PUNKNOWN        UnknownAdapter      OPTIONAL,
    IN      PRESOURCELIST   ResourceList,
    IN      PPORTDMUS       Port_,
    OUT     PSERVICEGROUP * ServiceGroup
)
{
    PAGED_CODE();

    UNREFERENCED_PARAMETER(UnknownAdapter);
    UNREFERENCED_PARAMETER(ResourceList);

    ASSERT(Port_);
    ASSERT(ServiceGroup);

    _DbgPrintF(DEBUGLVL_BLAB,("Init"));

    *ServiceGroup = NULL;
    m_fInitialized = FALSE;

    // This will remain unspecified if the miniport does not get any power
    // messages.
    //
    m_PowerState.DeviceState = PowerDeviceUnspecified;

    //
    // AddRef() is required because we are keeping this pointer.
    //
    m_pPort = Port_;
    m_pPort->AddRef();

    // Set dataformat.
    //
    if (IsEqualGUIDAligned(m_MusicFormatTechnology, GUID_NULL))
    {
        RtlCopyMemory(  &m_MusicFormatTechnology,
                        &KSMUSIC_TECHNOLOGY_PORT,
                        sizeof(GUID));
    }
    RtlCopyMemory(  &PinDataRangesStreamLegacy.Technology,
                    &m_MusicFormatTechnology,
                    sizeof(GUID));
    RtlCopyMemory(  &PinDataRangesStreamDMusic.Technology,
                    &m_MusicFormatTechnology,
                    sizeof(GUID));

    for (ULONG bufferCount = 0;bufferCount < kMidiInputBufferSize;bufferCount++)
    {
        m_MidiInputBuffer[bufferCount] = 0;
    }
    m_MidiInputBufferHead = 0;
    m_MidiInputBufferTail = 0;
    m_InputTimeStamp = 0;
    m_MasterClock = NULL;
    m_KSStateInput = KSSTATE_STOP;
    KeInitializeSpinLock(&m_InputBufferLock);

    NTSTATUS ntStatus = STATUS_SUCCESS;

    m_NumRenderStreams = 0;
    m_NumCaptureStreams = 0;

    ntStatus = PcNewServiceGroup(&m_pServiceGroup,NULL);
    if (NT_SUCCESS(ntStatus) && !m_pServiceGroup)   //  keep any error
    {
        ntStatus = STATUS_INSUFFICIENT_RESOURCES;
    }

    if (NT_SUCCESS(ntStatus))
    {
        *ServiceGroup = m_pServiceGroup;
        m_pServiceGroup->AddRef();

        //
        // Register the service group with the port early so the port is
        // prepared to handle notifications from the loopback path.
        //
        m_pPort->RegisterServiceGroup(m_pServiceGroup);
    }

    if (NT_SUCCESS(ntStatus))
    {
        m_fInitialized = TRUE;
    }

    if (!NT_SUCCESS(ntStatus))
    {
        //
        // clean up our mess
        //

        // clean up the service group
        if( m_pServiceGroup )
        {
            m_pServiceGroup->Release();
            m_pServiceGroup = NULL;
        }

        // clean up the out param service group.
        if (*ServiceGroup)
        {
            (*ServiceGroup)->Release();
            *ServiceGroup = NULL;
        }

        // release the port
        m_pPort->Release();
        m_pPort = NULL;
    }

    return ntStatus;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidi::NewStream()
 *****************************************************************************
 * Creates a new stream object for a pin instance.
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidi::
NewStream
(
    OUT     PMXF                  * MXF,
    IN      PUNKNOWN                OuterUnknown    OPTIONAL,
    _When_((PoolType & NonPagedPoolMustSucceed) != 0,
       __drv_reportError("Must succeed pool allocations are forbidden. "
                         "Allocation failures cause a system crash"))
    IN      POOL_TYPE               PoolType,
    IN      ULONG                   PinID,
    IN      DMUS_STREAM_TYPE        StreamType,
    IN      PKSDATAFORMAT           DataFormat,
    OUT     PSERVICEGROUP         * ServiceGroup,
    IN      PAllocatorMXF           AllocatorMXF,
    IN      PMASTERCLOCK            MasterClock,
    OUT     PULONGLONG              SchedulePreFetch
)
{
    PAGED_CODE();

    UNREFERENCED_PARAMETER(DataFormat);
    UNREFERENCED_PARAMETER(PinID);

    _DbgPrintF(DEBUGLVL_BLAB, ("NewStream"));
    NTSTATUS ntStatus = STATUS_SUCCESS;

    // In 100 ns, we want stuff as soon as it comes in
    //
    *SchedulePreFetch = 0;

    if  (   ((m_NumCaptureStreams < kMaxNumCaptureStreams)
            && (StreamType == DMUS_STREAM_MIDI_CAPTURE))
        ||  ((m_NumRenderStreams < kMaxNumLegacyRenderStreams + kMaxNumDMusicRenderStreams)
            && (StreamType == DMUS_STREAM_MIDI_RENDER))
        )
    {
        CMiniportRvfMidiStream *pStream =
            new(RvfMidiPoolFlagsFromPoolType(PoolType), MINMIDI_POOLTAG) CMiniportRvfMidiStream(OuterUnknown);

        if (pStream)
        {
            pStream->AddRef();

            ntStatus =
                pStream->Init(this,(StreamType == DMUS_STREAM_MIDI_CAPTURE),AllocatorMXF,MasterClock);

            if (NT_SUCCESS(ntStatus))
            {
                *MXF = PMXF(pStream);
                (*MXF)->AddRef();

                if (StreamType == DMUS_STREAM_MIDI_CAPTURE)
                {
                    m_NumCaptureStreams++;
                    *ServiceGroup = m_pServiceGroup;
                    (*ServiceGroup)->AddRef();
                }
                else
                {
                    m_NumRenderStreams++;
                    *ServiceGroup = NULL;
                }
            }

            pStream->Release();
        }
        else
        {
            ntStatus = STATUS_INSUFFICIENT_RESOURCES;
        }
    }
    else
    {
        ntStatus = STATUS_INVALID_DEVICE_REQUEST;
        if (StreamType == DMUS_STREAM_MIDI_CAPTURE)
        {
            _DbgPrintF(DEBUGLVL_TERSE,("NewStream failed, too many capture streams"));
        }
        else if (StreamType == DMUS_STREAM_MIDI_RENDER)
        {
            _DbgPrintF(DEBUGLVL_TERSE,("NewStream failed, too many render streams"));
        }
        else
        {
            _DbgPrintF(DEBUGLVL_TERSE,("NewStream invalid stream type"));
        }
    }

    return ntStatus;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidi::SetTechnology()
 *****************************************************************************
 * Sets pindatarange technology.
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidi::
SetTechnology
(
    _In_    const GUID *            Technology
)
{
    PAGED_CODE();

    NTSTATUS ntStatus = STATUS_UNSUCCESSFUL;

    // Fail if miniport has already been initialized.
    //
    if (NULL == m_pPort)
    {
        RtlCopyMemory(&m_MusicFormatTechnology, Technology, sizeof(GUID));
        ntStatus = STATUS_SUCCESS;
    }

    return ntStatus;
} // SetTechnology

/*****************************************************************************
 * CMiniportRvfMidi::PowerChangeNotify()
 *****************************************************************************
 * Handle power state change for the miniport.  Virtual device なので
 * hardware 再初期化は不要 — state bookkeeping のみ。
 */
#pragma code_seg("PAGE")
STDMETHODIMP_(void)
CMiniportRvfMidi::
PowerChangeNotify
(
    _In_    POWER_STATE             PowerState
)
{
    PAGED_CODE();

    _DbgPrintF(DEBUGLVL_VERBOSE, ("CMiniportRvfMidi::PowerChangeNotify D%d", PowerState.DeviceState));

    m_PowerState.DeviceState = PowerState.DeviceState;
} // PowerChangeNotify

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidiStream::NonDelegatingQueryInterface()
 *****************************************************************************
 * Obtains an interface.  This function works just like a COM QueryInterface
 * call and is used if the object is not being aggregated.
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidiStream::
NonDelegatingQueryInterface
(
    _In_           REFIID  Interface,
    _COM_Outptr_   PVOID * Object
)
{
    PAGED_CODE();

    _DbgPrintF(DEBUGLVL_BLAB,("Stream::NonDelegatingQueryInterface"));
    ASSERT(Object);

    if (IsEqualGUIDAligned(Interface,IID_IUnknown))
    {
        *Object = PVOID(PUNKNOWN(this));
    }
    else
    if (IsEqualGUIDAligned(Interface,IID_IMXF))
    {
        *Object = PVOID(PMXF(this));
    }
    else
    {
        *Object = NULL;
    }

    if (*Object)
    {
        //
        // We reference the interface for the caller.
        //
        PUNKNOWN(*Object)->AddRef();
        return STATUS_SUCCESS;
    }

    return STATUS_INVALID_PARAMETER;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidiStream::~CMiniportRvfMidiStream()
 *****************************************************************************
 * Destructs a stream.
 */
CMiniportRvfMidiStream::~CMiniportRvfMidiStream(void)
{
    PAGED_CODE();

    _DbgPrintF(DEBUGLVL_BLAB,("~CMiniportRvfMidiStream"));

    KeCancelTimer(&m_TimerEvent);

    if (m_DMKEvtQueue)
    {
        if (m_AllocatorMXF)
        {
            m_AllocatorMXF->PutMessage(m_DMKEvtQueue);
        }
        else
        {
            _DbgPrintF(DEBUGLVL_ERROR,("~CMiniportRvfMidiStream, no allocator, can't flush DMKEvts"));
        }
        m_DMKEvtQueue = NULL;
    }
    if (m_AllocatorMXF)
    {
        m_AllocatorMXF->Release();
        m_AllocatorMXF = NULL;
    }

    if (m_pMiniport)
    {
        if (m_fCapture)
        {
            m_pMiniport->m_NumCaptureStreams--;
        }
        else
        {
            m_pMiniport->m_NumRenderStreams--;
        }

        m_pMiniport->Release();
    }
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidiStream::Init()
 *****************************************************************************
 * Initializes a stream.
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidiStream::
Init
(
    IN      CMiniportRvfMidi *  pMiniport,
    IN      BOOLEAN             fCapture,
    IN      PAllocatorMXF       allocatorMXF,
    IN      PMASTERCLOCK        masterClock
)
{
    PAGED_CODE();

    ASSERT(pMiniport);

    _DbgPrintF(DEBUGLVL_BLAB,("Init"));

    m_TimerQueued = FALSE;
    KeInitializeSpinLock(&m_DpcSpinLock);
    m_pMiniport = pMiniport;
    m_pMiniport->AddRef();

    pMiniport->m_MasterClock = masterClock;

    m_fCapture = fCapture;

    m_SnapshotTimeStamp = 0;
    m_DMKEvtQueue = NULL;
    m_DMKEvtOffset = 0;

    m_NumberOfRetries = 0;

    if (allocatorMXF)
    {
        allocatorMXF->AddRef();
        m_AllocatorMXF = allocatorMXF;
        m_sinkMXF = m_AllocatorMXF;
    }
    else
    {
        return STATUS_INVALID_PARAMETER;
    }

    KeInitializeDpc
    (
        &m_Dpc,
        &::RvfMidiTimerDPC,
        PVOID(this)
    );
    KeInitializeTimer(&m_TimerEvent);

    return STATUS_SUCCESS;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidiStream::SetState()
 *****************************************************************************
 * Sets the state of the channel.
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidiStream::
SetState
(
    _In_    KSSTATE     NewState
)
{
    PAGED_CODE();

    _DbgPrintF(DEBUGLVL_VERBOSE,("SetState %d",NewState));

    if (NewState == KSSTATE_RUN)
    {
        if (m_pMiniport->m_fInitialized)
        {
            LARGE_INTEGER timeDue100ns;
            timeDue100ns.QuadPart = 0;
            KeSetTimer(&m_TimerEvent,timeDue100ns,&m_Dpc);
        }
        else
        {
            _DbgPrintF(DEBUGLVL_TERSE, ("CMiniportRvfMidiStream::SetState KSSTATE_RUN failed due to uninitialized miniport"));
            return STATUS_INVALID_DEVICE_STATE;
        }
    }

    if (m_fCapture)
    {
        m_pMiniport->m_KSStateInput = NewState;
        if (NewState == KSSTATE_STOP)   //  STOPping
        {
            // SetState は PASSIVE 呼出 — raise-to-DISPATCH variant を使う。
            KIRQL oldIrql;
            KeAcquireSpinLock(&m_pMiniport->m_InputBufferLock, &oldIrql);
            m_pMiniport->m_MidiInputBufferHead = 0;   // Previously read bytes are discarded.
            m_pMiniport->m_MidiInputBufferTail = 0;   // The entire FIFO is available.
            m_pMiniport->m_InputTimeStamp = 0;
            KeReleaseSpinLock(&m_pMiniport->m_InputBufferLock, oldIrql);
        }
    }
    return STATUS_SUCCESS;
}

#pragma code_seg()
/*****************************************************************************
 * CMiniportRvfMidi::Service()
 *****************************************************************************
 * DPC-mode service call from the port.
 */
STDMETHODIMP_(void)
CMiniportRvfMidi::
Service
(   void
)
{
    _DbgPrintF(DEBUGLVL_BLAB, ("Service"));
    if (!m_NumCaptureStreams)
    {
        //  we should never get here....
        //  if we do, we must have read some trash,
        //  so just reset the input FIFO
        KeAcquireSpinLockAtDpcLevel(&m_InputBufferLock);
        m_MidiInputBufferTail = m_MidiInputBufferHead = 0;
        m_InputTimeStamp = 0;
        KeReleaseSpinLockFromDpcLevel(&m_InputBufferLock);
    }
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidiStream::ConnectOutput()
 *****************************************************************************
 * Connects the upstream sink to a capture stream.
 */
NTSTATUS
CMiniportRvfMidiStream::
ConnectOutput(_In_  PMXF sinkMXF)
{
    PAGED_CODE();

    if (m_fCapture)
    {
        if ((sinkMXF) && (m_sinkMXF == m_AllocatorMXF))
        {
            _DbgPrintF(DEBUGLVL_BLAB, ("ConnectOutput"));
            m_sinkMXF = sinkMXF;
            return STATUS_SUCCESS;
        }
        else
        {
            _DbgPrintF(DEBUGLVL_TERSE, ("ConnectOutput failed"));
        }
    }
    else
    {
        _DbgPrintF(DEBUGLVL_TERSE, ("ConnectOutput called on renderer; failed"));
    }
    return STATUS_UNSUCCESSFUL;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidiStream::DisconnectOutput()
 *****************************************************************************
 * Disconnects the upstream sink from a capture stream.
 */
NTSTATUS
CMiniportRvfMidiStream::
DisconnectOutput(_In_   PMXF sinkMXF)
{
    PAGED_CODE();

    if (m_fCapture)
    {
        if ((m_sinkMXF == sinkMXF) || (!sinkMXF))
        {
            _DbgPrintF(DEBUGLVL_BLAB, ("DisconnectOutput"));
            m_sinkMXF = m_AllocatorMXF;
            return STATUS_SUCCESS;
        }
        else
        {
            _DbgPrintF(DEBUGLVL_TERSE, ("DisconnectOutput failed"));
        }
    }
    else
    {
        _DbgPrintF(DEBUGLVL_TERSE, ("DisconnectOutput called on renderer; failed"));
    }
    return STATUS_UNSUCCESSFUL;
}

#pragma code_seg()
/*****************************************************************************
 * CMiniportRvfMidiStream::PutMessage()
 *****************************************************************************
 * Writes an outgoing MIDI message.
 * We don't sort a new message into the queue -- we append it.
 * This is fine, since the sequencer feeds us sequenced data.
 * Timestamps will ascend by design.
 */
NTSTATUS CMiniportRvfMidiStream::PutMessage(_In_   PDMUS_KERNEL_EVENT pDMKEvt)
{
    NTSTATUS            ntStatus = STATUS_SUCCESS;
    PDMUS_KERNEL_EVENT  aDMKEvt;

    ASSERT(KeGetCurrentIrql() == DISPATCH_LEVEL);

    if (!m_fCapture)
    {
        _DbgPrintF(DEBUGLVL_BLAB, ("PutMessage to render stream"));
        if (pDMKEvt)
        {
            KeAcquireSpinLockAtDpcLevel(&m_DpcSpinLock);

            if (m_DMKEvtQueue)
            {
                aDMKEvt = m_DMKEvtQueue;            //  put pDMKEvt in event queue

                while (aDMKEvt->pNextEvt)
                {
                    aDMKEvt = aDMKEvt->pNextEvt;
                }
                aDMKEvt->pNextEvt = pDMKEvt;        //  here is end of queue
            }
            else                                    //  currently nothing in queue
            {
                m_DMKEvtQueue = pDMKEvt;
                if (m_DMKEvtOffset)
                {
                    _DbgPrintF(DEBUGLVL_ERROR, ("PutMessage  Nothing in the queue, but m_DMKEvtOffset == %d",m_DMKEvtOffset));
                    m_DMKEvtOffset = 0;
                }
            }

            KeReleaseSpinLockFromDpcLevel(&m_DpcSpinLock);
        }
        if (!m_TimerQueued)
        {
            (void) ConsumeEvents();
        }
    }
    else    //  capture
    {
        _DbgPrintF(DEBUGLVL_BLAB, ("PutMessage to capture stream"));
        ASSERT(NULL == pDMKEvt);

        SourceEvtsToPort();
    }
    return ntStatus;
}

#pragma code_seg()
/*****************************************************************************
 * CMiniportRvfMidiStream::PutMessageLocked()
 *****************************************************************************
 * Now that the render stream's m_DpcSpinLock is held, add this message
 * to the queue.  Called from ConsumeEvents' package branch.
 * We don't sort a new message into the queue -- we append it.
 * upstream は末尾で !m_TimerQueued なら ConsumeEvents() を再帰呼出しするが、
 * ここでは m_DpcSpinLock を保持したままなので再取得で deadlock する —
 * enqueue のみ行い、drain は呼出元の ConsumeEvents loop に任せる。
 */
NTSTATUS CMiniportRvfMidiStream::PutMessageLocked(PDMUS_KERNEL_EVENT pDMKEvt)
{
    NTSTATUS            ntStatus = STATUS_SUCCESS;
    PDMUS_KERNEL_EVENT  aDMKEvt;

    ASSERT(KeGetCurrentIrql() == DISPATCH_LEVEL);

    if (!m_fCapture)
    {
        _DbgPrintF(DEBUGLVL_BLAB, ("PutMessage to render stream"));
        if (pDMKEvt)
        {
            // m_DpcSpinLock already held

            if (m_DMKEvtQueue)
            {
                aDMKEvt = m_DMKEvtQueue;            //  put pDMKEvt in event queue

                while (aDMKEvt->pNextEvt)
                {
                    aDMKEvt = aDMKEvt->pNextEvt;
                }
                aDMKEvt->pNextEvt = pDMKEvt;        //  here is end of queue
            }
            else                                    //  currently nothing in queue
            {
                m_DMKEvtQueue = pDMKEvt;
                if (m_DMKEvtOffset)
                {
                    _DbgPrintF(DEBUGLVL_ERROR, ("PutMessage  Nothing in the queue, but m_DMKEvtOffset == %d",m_DMKEvtOffset));
                    m_DMKEvtOffset = 0;
                }
            }

            // m_DpcSpinLock already held
        }
    }
    else    //  capture
    {
        _DbgPrintF(DEBUGLVL_BLAB, ("PutMessage to capture stream"));
        ASSERT(NULL == pDMKEvt);

        SourceEvtsToPort();
    }
    return ntStatus;
}

#pragma code_seg()
/*****************************************************************************
 * CMiniportRvfMidiStream::ConsumeEvents()
 *****************************************************************************
 * Attempts to empty the render message queue.
 * Called either from DPC timer or upon IRP submittal.
 */
NTSTATUS CMiniportRvfMidiStream::ConsumeEvents(void)
{
    PDMUS_KERNEL_EVENT aDMKEvt;

    NTSTATUS        ntStatus = STATUS_SUCCESS;
    ULONG           bytesRemaining = 0,bytesWritten = 0;
    LARGE_INTEGER   aMillisecIn100ns;

    ASSERT(KeGetCurrentIrql() == DISPATCH_LEVEL);
    KeAcquireSpinLockAtDpcLevel(&m_DpcSpinLock);

    m_TimerQueued = FALSE;
    while (m_DMKEvtQueue)                   //  do we have anything to play at all?
    {
        aDMKEvt = m_DMKEvtQueue;                            //  event we try to play
        if (aDMKEvt->cbEvent)
        {
            bytesRemaining = aDMKEvt->cbEvent - m_DMKEvtOffset; //  number of bytes left in this evt

            ASSERT(bytesRemaining > 0);
            if (bytesRemaining <= 0)
            {
                bytesRemaining = aDMKEvt->cbEvent;
            }

            if (aDMKEvt->cbEvent <= sizeof(PBYTE))                //  short message
            {
                _DbgPrintF(DEBUGLVL_BLAB, ("ConsumeEvents(%02x%02x%02x%02x)",aDMKEvt->uData.abData[0],aDMKEvt->uData.abData[1],aDMKEvt->uData.abData[2],aDMKEvt->uData.abData[3]));
                ntStatus = Write(aDMKEvt->uData.abData + m_DMKEvtOffset,bytesRemaining,&bytesWritten);
            }
            else if (PACKAGE_EVT(aDMKEvt))
            {
                ASSERT(m_DMKEvtOffset == 0);
                m_DMKEvtOffset = 0;
                _DbgPrintF(DEBUGLVL_BLAB, ("ConsumeEvents(Package)"));

                ntStatus = PutMessageLocked(aDMKEvt->uData.pPackageEvt);  // we already own the spinlock

                // null this because we are about to throw it in the allocator
                aDMKEvt->uData.pPackageEvt = NULL;
                aDMKEvt->cbEvent = 0;
                bytesWritten = bytesRemaining;
            }
            else                //  SysEx message
            {
                _DbgPrintF(DEBUGLVL_BLAB, ("ConsumeEvents(%02x%02x%02x%02x)",aDMKEvt->uData.pbData[0],aDMKEvt->uData.pbData[1],aDMKEvt->uData.pbData[2],aDMKEvt->uData.pbData[3]));

                ntStatus = Write(aDMKEvt->uData.pbData + m_DMKEvtOffset,bytesRemaining,&bytesWritten);
            }
        }   //  if (aDMKEvt->cbEvent)
        if (STATUS_SUCCESS != ntStatus)
        {
            _DbgPrintF(DEBUGLVL_TERSE, ("ConsumeEvents: Write returned 0x%08x",ntStatus));
            bytesWritten = bytesRemaining;  //  just bail on this event and try next time
        }

        ASSERT(bytesWritten <= bytesRemaining);
        if (bytesWritten == bytesRemaining)
        {
            m_DMKEvtQueue = m_DMKEvtQueue->pNextEvt;
            aDMKEvt->pNextEvt = NULL;

            m_AllocatorMXF->PutMessage(aDMKEvt);    //  throw back in free pool
            m_DMKEvtOffset = 0;                     //  start fresh on next evt
            m_NumberOfRetries = 0;
        }           //  but wait ... there's more!
        else        //  our FIFO is full for now.
        {
            //  update our offset by that amount we did write
            m_DMKEvtOffset += bytesWritten;
            ASSERT(m_DMKEvtOffset < aDMKEvt->cbEvent);

            _DbgPrintF(DEBUGLVL_BLAB,("ConsumeEvents tried %d, wrote %d, at offset %d",bytesRemaining,bytesWritten,m_DMKEvtOffset));
            aMillisecIn100ns.QuadPart = -(kOneMillisec);    //  set timer, come back later
            m_TimerQueued = TRUE;
            m_NumberOfRetries++;
            KeSetTimer( &m_TimerEvent, aMillisecIn100ns, &m_Dpc );
            break;
        }   //  we didn't write it all
    }       //  go back, Jack, do it again (while m_DMKEvtQueue)
    KeReleaseSpinLockFromDpcLevel(&m_DpcSpinLock);
    return ntStatus;
}

#pragma code_seg("PAGE")
/*****************************************************************************
 * CMiniportRvfMidiStream::HandlePortParams()
 *****************************************************************************
 * Handles the SYNTH_PORTPARAMS property request.
 */
NTSTATUS
CMiniportRvfMidiStream::
HandlePortParams
(
    IN      PPCPROPERTY_REQUEST     pRequest
)
{
    PAGED_CODE();

    NTSTATUS ntStatus;

    if (pRequest->Verb & KSPROPERTY_TYPE_SET)
    {
        return STATUS_INVALID_DEVICE_REQUEST;
    }

    ntStatus = ValidatePropertyRequest(pRequest, sizeof(SYNTH_PORTPARAMS), TRUE);
    if (NT_SUCCESS(ntStatus))
    {
        RtlCopyMemory(pRequest->Value, pRequest->Instance, sizeof(SYNTH_PORTPARAMS));

        PSYNTH_PORTPARAMS Params = (PSYNTH_PORTPARAMS)pRequest->Value;

        if (Params->ValidParams & ~SYNTH_PORTPARAMS_CHANNELGROUPS)
        {
            Params->ValidParams &= SYNTH_PORTPARAMS_CHANNELGROUPS;
        }

        if (!(Params->ValidParams & SYNTH_PORTPARAMS_CHANNELGROUPS))
        {
            Params->ChannelGroups = 1;
        }
        else if (Params->ChannelGroups != 1)
        {
            Params->ChannelGroups = 1;
        }

        pRequest->ValueSize = sizeof(SYNTH_PORTPARAMS);
    }

    return ntStatus;
}

#pragma code_seg()
/*****************************************************************************
 * RvfMidiTimerDPC()
 *****************************************************************************
 * The timer DPC callback. Thunks to a C++ member function.
 * This is called by the OS in response to the DirectMusic pin
 * wanting to wakeup later to process more DirectMusic stuff.
 */
VOID
NTAPI
RvfMidiTimerDPC
(
    IN  PKDPC   Dpc,
    IN  PVOID   DeferredContext,
    IN  PVOID   SystemArgument1,
    IN  PVOID   SystemArgument2
)
{
    ASSERT(DeferredContext);

    UNREFERENCED_PARAMETER(Dpc);
    UNREFERENCED_PARAMETER(SystemArgument1);
    UNREFERENCED_PARAMETER(SystemArgument2);

    CMiniportRvfMidiStream *aStream;
    aStream = (CMiniportRvfMidiStream *) DeferredContext;
    if (aStream)
    {
        _DbgPrintF(DEBUGLVL_BLAB,("RvfMidiTimerDPC"));
        if (false == aStream->m_fCapture)
        {
            (void) aStream->ConsumeEvents();
        }
        //  ignores return value!
    }
}

/*****************************************************************************
 * DirectMusic properties
 ****************************************************************************/

#pragma code_seg("PAGE")
/*
 *  Properties concerning synthesizer functions.
 */
const WCHAR wszDescOut[] = L"Ruviform MIDI Loopback";
const WCHAR wszDescIn[]  = L"Ruviform MIDI Loopback";

NTSTATUS PropertyHandler_Synth
(
    IN      PPCPROPERTY_REQUEST     pRequest
)
{
    NTSTATUS    ntStatus;

    PAGED_CODE();

    if (pRequest->Verb & KSPROPERTY_TYPE_BASICSUPPORT)
    {
        ntStatus = ValidatePropertyRequest(pRequest, sizeof(ULONG), TRUE);
        if (NT_SUCCESS(ntStatus))
        {
            // if return buffer can hold a ULONG, return the access flags
            PULONG AccessFlags = PULONG(pRequest->Value);

            *AccessFlags = KSPROPERTY_TYPE_BASICSUPPORT;
            switch (pRequest->PropertyItem->Id)
            {
                case KSPROPERTY_SYNTH_CAPS:
                case KSPROPERTY_SYNTH_CHANNELGROUPS:
                    *AccessFlags |= KSPROPERTY_TYPE_GET;
            }
            switch (pRequest->PropertyItem->Id)
            {
                case KSPROPERTY_SYNTH_CHANNELGROUPS:
                    *AccessFlags |= KSPROPERTY_TYPE_SET;
            }
            ntStatus = STATUS_SUCCESS;
            pRequest->ValueSize = sizeof(ULONG);

            switch (pRequest->PropertyItem->Id)
            {
                case KSPROPERTY_SYNTH_PORTPARAMETERS:
                    if (pRequest->MinorTarget)
                    {
                        *AccessFlags |= KSPROPERTY_TYPE_GET;
                    }
                    else
                    {
                        pRequest->ValueSize = 0;
                        ntStatus = STATUS_INVALID_DEVICE_REQUEST;
                    }
            }
        }
    }
    else
    {
        ntStatus = STATUS_SUCCESS;
        switch(pRequest->PropertyItem->Id)
        {
            case KSPROPERTY_SYNTH_CAPS:
                _DbgPrintF(DEBUGLVL_VERBOSE,("PropertyHandler_Synth:KSPROPERTY_SYNTH_CAPS"));

                if (pRequest->Verb & KSPROPERTY_TYPE_SET)
                {
                    ntStatus = STATUS_INVALID_DEVICE_REQUEST;
                }

                if (NT_SUCCESS(ntStatus))
                {
                    ntStatus = ValidatePropertyRequest(pRequest, sizeof(SYNTHCAPS), TRUE);

                    if (NT_SUCCESS(ntStatus))
                    {
                        SYNTHCAPS *caps = (SYNTHCAPS*)pRequest->Value;
                        int increment;
                        RtlZeroMemory(caps, sizeof(SYNTHCAPS));
                        //
                        if (pRequest->Node == eSynthNode)
                        {
                            increment = sizeof(wszDescOut) - 2;
                            RtlCopyMemory( caps->Description,wszDescOut,increment);
                            caps->Guid           = CLSID_MiniportDriverDMusUART;
                        }
                        else
                        {
                            increment = sizeof(wszDescIn) - 2;
                            RtlCopyMemory( caps->Description,wszDescIn,increment);
                            caps->Guid           = CLSID_MiniportDriverDMusUARTCapture;
                        }

                        caps->Flags              = SYNTH_PC_EXTERNAL;
                        caps->MemorySize         = 0;
                        caps->MaxChannelGroups   = 1;
                        caps->MaxVoices          = 0xFFFFFFFF;
                        caps->MaxAudioChannels   = 0xFFFFFFFF;

                        caps->EffectFlags        = 0;

                        pRequest->ValueSize = sizeof(SYNTHCAPS);
                    }
                }
                break;

             case KSPROPERTY_SYNTH_PORTPARAMETERS:
                _DbgPrintF(DEBUGLVL_VERBOSE,("PropertyHandler_Synth:KSPROPERTY_SYNTH_PORTPARAMETERS"));
    {
                CMiniportRvfMidiStream *aStream;

                aStream = (CMiniportRvfMidiStream*)(pRequest->MinorTarget);
                if (aStream)
                {
                    ntStatus = aStream->HandlePortParams(pRequest);
                }
                else
                {
                    ntStatus = STATUS_INVALID_DEVICE_REQUEST;
                }
               }
               break;

            case KSPROPERTY_SYNTH_CHANNELGROUPS:
                _DbgPrintF(DEBUGLVL_VERBOSE,("PropertyHandler_Synth:KSPROPERTY_SYNTH_CHANNELGROUPS"));

                ntStatus = ValidatePropertyRequest(pRequest, sizeof(ULONG), TRUE);
                if (NT_SUCCESS(ntStatus))
                {
                    *(PULONG)(pRequest->Value) = 1;
                    pRequest->ValueSize = sizeof(ULONG);
                }
                break;

            case KSPROPERTY_SYNTH_LATENCYCLOCK:
                _DbgPrintF(DEBUGLVL_VERBOSE,("PropertyHandler_Synth:KSPROPERTY_SYNTH_LATENCYCLOCK"));

                if(pRequest->Verb & KSPROPERTY_TYPE_SET)
                {
                    ntStatus = STATUS_INVALID_DEVICE_REQUEST;
                }
                else
                {
                    ntStatus = ValidatePropertyRequest(pRequest, sizeof(ULONGLONG), TRUE);
                    if(NT_SUCCESS(ntStatus))
                    {
                        REFERENCE_TIME rtLatency;
                        CMiniportRvfMidiStream *aStream;

                        aStream = (CMiniportRvfMidiStream*)(pRequest->MinorTarget);
                        if(aStream == NULL)
                        {
                            ntStatus = STATUS_INVALID_DEVICE_REQUEST;
                        }
                        else if (aStream->m_pMiniport->m_MasterClock)
                        {
                            ntStatus = aStream->m_pMiniport->m_MasterClock->GetTime(&rtLatency);
                            *((PULONGLONG)pRequest->Value) = rtLatency;
                            pRequest->ValueSize = sizeof(ULONGLONG);
                        }
                        else
                        {
                            *((PULONGLONG)pRequest->Value) = 0;
                            pRequest->ValueSize = sizeof(ULONGLONG);
                        }
                    }
                }
                break;

            default:
                _DbgPrintF(DEBUGLVL_TERSE,("Unhandled property in PropertyHandler_Synth"));
                break;
        }
    }
    return ntStatus;
}

/*****************************************************************************
 * ValidatePropertyRequest()
 *****************************************************************************
 * Validates pRequest.
 *  Checks if the ValueSize is valid
 *  Checks if the Value is valid
 *
 *  This does not update pRequest->ValueSize if it returns NT_SUCCESS.
 *  Caller must set pRequest->ValueSize in case of NT_SUCCESS.
 */
NTSTATUS ValidatePropertyRequest
(
    IN      PPCPROPERTY_REQUEST     pRequest,
    IN      ULONG                   ulValueSize,
    IN      BOOLEAN                 fValueRequired
)
{
    PAGED_CODE();

    NTSTATUS    ntStatus;

    if (pRequest->ValueSize >= ulValueSize)
    {
        if (fValueRequired && NULL == pRequest->Value)
        {
            ntStatus = STATUS_INVALID_PARAMETER;
        }
        else
        {
            ntStatus = STATUS_SUCCESS;
        }
    }
    else  if (0 == pRequest->ValueSize)
    {
        ntStatus = STATUS_BUFFER_OVERFLOW;
    }
    else
    {
        ntStatus = STATUS_BUFFER_TOO_SMALL;
    }

    if (STATUS_BUFFER_OVERFLOW == ntStatus)
    {
        pRequest->ValueSize = ulValueSize;
    }
    else
    {
        pRequest->ValueSize = 0;
    }

    return ntStatus;
} // ValidatePropertyRequest

#pragma code_seg()
