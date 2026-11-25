/*****************************************************************************
 * RvfMidi.h - Ruviform virtual MIDI loopback miniport private definitions
 *****************************************************************************
 * Copyright (c) 1997-2000 Microsoft Corporation.  All Rights Reserved.
 *
 * ruviform 派生版 — WDK 8.1 "DirectMusic UART Driver Sample" (DMusUART) の
 * private.h 由来。MPU-401 ハードウェア UART I/O を software render→capture
 * loopback に置き換えた（出所・変更点は PROVENANCE.md を参照）。
 */

#ifndef _RVF_MIDI_PRIVATE_H_
#define _RVF_MIDI_PRIVATE_H_

#include <portcls.h>
#include <ntstrsafe.h>
#include <stdunk.h>
#include <dmusicks.h>
#include "NewDelete.h"

//  + for absolute / - for relative
#define kOneMillisec (10 * 1000)

// MIDI miniport pool tag ("RMId")
#define MINMIDI_POOLTAG 'dIMR'

// WinMM midiIn/midiOut で同名ポートを列挙させるため、render/capture 両 pin
// の pin-name GUID を同一の独自 GUID に固定する。表示文字列は INF の
// MediaCategories 登録で "Ruviform MIDI Loopback" に結び付ける。
// 定義は RvfMidiMiniport.cpp に 1 箇所のみ置く（initguid 経由にすると
// dmusicks.h 全体の GUID が common.cpp TU で duplicate 定義になるため、
// この GUID は DEFINE_GUID/initguid 機構の外で管理する）。
// {714b96c3-8003-41c2-b673-1b086cbdb3db}
EXTERN_C const GUID FAR RVF_MIDI_LOOPBACK_NAME;

/*****************************************************************************
 * Prototypes
 */

NTSTATUS ValidatePropertyRequest(IN PPCPROPERTY_REQUEST pRequest, IN ULONG ulValueSize, IN BOOLEAN fValueRequired);

NTSTATUS PropertyHandler_Synth(IN PPCPROPERTY_REQUEST pRequest);

VOID NTAPI RvfMidiTimerDPC(IN PKDPC, IN PVOID, IN PVOID, IN PVOID);

/*****************************************************************************
 * Constants
 */

const LONG      kMidiInputBufferSize = 128;

/*****************************************************************************
 * Helpers
 */

// IMiniportDMus::NewStream は legacy POOL_TYPE を渡す — NewDelete.h の
// operator new は POOL_FLAGS 取りなので変換する。DMus stream は
// DISPATCH_LEVEL で触れるため non-paged 必須 — PagedPool 系のみ paged と
// みなし、それ以外（NonPagedPool/Nx/CacheAligned/MustSucceed 等）は全て
// non-paged に寄せる。
inline POOL_FLAGS RvfMidiPoolFlagsFromPoolType(_In_ POOL_TYPE PoolType)
{
    switch (PoolType)
    {
        case PagedPool:
        case PagedPoolCacheAligned:
            return POOL_FLAG_PAGED;
        default:
            return POOL_FLAG_NON_PAGED;
    }
}

/*****************************************************************************
 * Classes
 */

class CMiniportRvfMidiStream;

/*****************************************************************************
 * CMiniportRvfMidi
 *****************************************************************************
 * Virtual MIDI loopback miniport.  This object is associated with the
 * device and is created when the device is started.  The class inherits
 * IMiniportDMus so it can expose this interface and CUnknown so it
 * automatically gets reference counting and aggregation support.
 */
class CMiniportRvfMidi
:   public IMiniportDMus,
    public IMusicTechnology,
    public IPowerNotify,
    public CUnknown
{
private:
    KSSTATE         m_KSStateInput;         // Miniport state (RUN/PAUSE/ACQUIRE/STOP)
    PPORTDMUS       m_pPort;                // Callback interface.
    PSERVICEGROUP   m_pServiceGroup;        // Service group for capture.
    PMASTERCLOCK    m_MasterClock;          // for input data
    REFERENCE_TIME  m_InputTimeStamp;       // capture data timestamp
    USHORT          m_NumRenderStreams;     // Num active render streams.
    USHORT          m_NumCaptureStreams;    // Num active capture streams.
    LONG            m_MidiInputBufferHead;  // Index of the newest byte in the FIFO.
    LONG            m_MidiInputBufferTail;  // Index of the oldest empty space in the FIFO.
    KSPIN_LOCK      m_InputBufferLock;      // Serializes FIFO head/tail access.
    GUID            m_MusicFormatTechnology;
    POWER_STATE     m_PowerState;           // Saved power state (D0 = full power, D3 = off)
    BOOLEAN         m_fInitialized;         // Is the miniport initialized.
    UCHAR           m_MidiInputBuffer[kMidiInputBufferSize];  // Internal SW FIFO.

public:
    /*************************************************************************
     * The following two macros are from STDUNK.H.  DECLARE_STD_UNKNOWN()
     * defines inline IUnknown implementations that use CUnknown's aggregation
     * support.  NonDelegatingQueryInterface() is declared, but it cannot be
     * implemented generically.  Its definition appears in RvfMidiMiniport.cpp.
     * DEFINE_STD_CONSTRUCTOR() defines inline a constructor which accepts
     * only the outer unknown, which is used for aggregation.  The standard
     * create macro (in RvfMidiMiniport.cpp) uses this constructor.
     */
    DECLARE_STD_UNKNOWN();
    DEFINE_STD_CONSTRUCTOR(CMiniportRvfMidi);

    ~CMiniportRvfMidi();

    /*************************************************************************
     * IMiniport methods
     */
    STDMETHODIMP_(NTSTATUS)
    GetDescription
    (   OUT     PPCFILTER_DESCRIPTOR *  OutFilterDescriptor
    );
    STDMETHODIMP_(NTSTATUS)
    DataRangeIntersection
    (   IN      ULONG           PinId
    ,   IN      PKSDATARANGE    DataRange
    ,   IN      PKSDATARANGE    MatchingDataRange
    ,   IN      ULONG           OutputBufferLength
    ,   OUT     PVOID           ResultantFormat
    ,   OUT     PULONG          ResultantFormatLength
    )
    {
        UNREFERENCED_PARAMETER(PinId);
        UNREFERENCED_PARAMETER(DataRange);
        UNREFERENCED_PARAMETER(MatchingDataRange);
        UNREFERENCED_PARAMETER(OutputBufferLength);
        UNREFERENCED_PARAMETER(ResultantFormat);
        UNREFERENCED_PARAMETER(ResultantFormatLength);

        return STATUS_NOT_IMPLEMENTED;
    }

    /*************************************************************************
     * IMiniportDMus methods
     */
    STDMETHODIMP_(NTSTATUS) Init
    (
        IN      PUNKNOWN        UnknownAdapter,
        IN      PRESOURCELIST   ResourceList,
        IN      PPORTDMUS       Port,
        OUT     PSERVICEGROUP * ServiceGroup
    );
    STDMETHODIMP_(NTSTATUS) NewStream
    (
        OUT     PMXF                  * Stream,
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
    );
    STDMETHODIMP_(void) Service
    (   void
    );

    /*************************************************************************
     * IMusicTechnology methods
     */
    IMP_IMusicTechnology;

    /*************************************************************************
     * IPowerNotify methods
     */
    IMP_IPowerNotify;

    /*************************************************************************
     * Loopback entry point — render stream の Write() から呼ばれ、byte 列を
     * capture FIFO へ積んで port に notify する。
     */
    NTSTATUS FeedCaptureFifo
    (
        IN      PUCHAR      Data,
        IN      ULONG       Length,
        OUT     PULONG      BytesWritten
    );

    /*************************************************************************
     * Friends
     */
    friend class CMiniportRvfMidiStream;
    friend NTSTATUS PropertyHandler_Synth(IN PPCPROPERTY_REQUEST);
};

/*****************************************************************************
 * CMiniportRvfMidiStream
 *****************************************************************************
 * Virtual MIDI loopback miniport stream.  This object is associated with
 * the pin and is created when the pin is instantiated.  It inherits IMXF
 * so it can expose this interface and CUnknown so it automatically gets
 * reference counting and aggregation support.
 */
class CMiniportRvfMidiStream
:   public IMXF,
    public CUnknown
{
private:
    CMiniportRvfMidi *  m_pMiniport;            // Parent.
    REFERENCE_TIME      m_SnapshotTimeStamp;    // Current snapshot of miniport's input timestamp.
    BOOLEAN             m_fCapture;             // Whether this is capture.
    PAllocatorMXF       m_AllocatorMXF;         // source/sink for DMus structs
    PMXF                m_sinkMXF;              // sink for DMus capture
    PDMUS_KERNEL_EVENT  m_DMKEvtQueue;          // queue of waiting events
    ULONG               m_NumberOfRetries;      // Number of consecutive times the fifo was full
    ULONG               m_DMKEvtOffset;         // offset into the event
    KDPC                m_Dpc;                  // DPC for timer
    KTIMER              m_TimerEvent;           // timer
    BOOL                m_TimerQueued;          // whether a timer has been set
    KSPIN_LOCK          m_DpcSpinLock;          // protects the ConsumeEvents DPC

    STDMETHODIMP_(NTSTATUS) SourceEvtsToPort();
    STDMETHODIMP_(NTSTATUS) ConsumeEvents();
    STDMETHODIMP_(NTSTATUS) PutMessageLocked(PDMUS_KERNEL_EVENT pDMKEvt);

public:
    /*************************************************************************
     * The following two macros are from STDUNK.H.  DECLARE_STD_UNKNOWN()
     * defines inline IUnknown implementations that use CUnknown's aggregation
     * support.  NonDelegatingQueryInterface() is declared, but it cannot be
     * implemented generically.  Its definition appears in RvfMidiMiniport.cpp.
     * DEFINE_STD_CONSTRUCTOR() defines inline a constructor which accepts
     * only the outer unknown, which is used for aggregation.  The standard
     * create macro (in RvfMidiMiniport.cpp) uses this constructor.
     */
    DECLARE_STD_UNKNOWN();
    DEFINE_STD_CONSTRUCTOR(CMiniportRvfMidiStream);

    ~CMiniportRvfMidiStream();

    STDMETHODIMP_(NTSTATUS) Init
    (
        IN      CMiniportRvfMidi *  pMiniport,
        IN      BOOLEAN             fCapture,
        IN      PAllocatorMXF       allocatorMXF,
        IN      PMASTERCLOCK        masterClock
    );

    NTSTATUS HandlePortParams
    (
        IN      PPCPROPERTY_REQUEST     Request
    );

    /*************************************************************************
     * IMXF methods
     */
    IMP_IMXF;

    STDMETHODIMP_(NTSTATUS) Write
    (
        IN      PVOID       BufferAddress,
        IN      ULONG       BytesToWrite,
        OUT     PULONG      BytesWritten
    );

    friend VOID NTAPI
    RvfMidiTimerDPC
    (
        IN      PKDPC   Dpc,
        IN      PVOID   DeferredContext,
        IN      PVOID   SystemArgument1,
        IN      PVOID   SystemArgument2
    );
    friend NTSTATUS PropertyHandler_Synth(IN PPCPROPERTY_REQUEST);
};

// common.h の forward declaration と同型（InstallSubdevice が
// PFNCREATEMINIPORT 経由で渡すパラメータ型 — 本 factory では未使用）
typedef struct _ENDPOINT_MINIPAIR *PENDPOINT_MINIPAIR;

// PFNCREATEMINIPORT 形の factory — adapter.cpp の InstallSubdevice 呼出で
// CLSID_PortDMus port に対して本 miniport を結び付ける。
NTSTATUS
CreateMiniportRvfMidi
(
    _Out_           PUNKNOWN *                          Unknown,
    _In_            REFCLSID                            ClassId,
    _In_opt_        PUNKNOWN                            UnknownOuter,
    _In_            POOL_FLAGS                          PoolFlags,
    _In_            PUNKNOWN                            UnknownAdapter,
    _In_opt_        PVOID                               DeviceContext,
    _In_            PENDPOINT_MINIPAIR                  MiniportPair
);

#endif  //  _RVF_MIDI_PRIVATE_H_
