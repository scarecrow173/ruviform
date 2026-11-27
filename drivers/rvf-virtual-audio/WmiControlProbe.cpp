/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    WmiControlProbe.cpp

Abstract:

    rf#1034 diagnostic-only driver-owned WMI control/callsite probe.

--*/

#include <initguid.h>
#include <sysvad.h>
#include <wmistr.h>
#include <wmilib.h>
#include <evntrace.h>
#include "WmiControlProbe.h"

DEFINE_GUID(RVF1034_WMI_PROBE_CONTROL_GUID,
    0x6f1d2b3a, 0x1034, 0x4a7f, 0x9e, 0x2c, 0x5a, 0x11, 0xc0, 0xff, 0x21, 0x01);
DEFINE_GUID(RVF1034_WMI_PROBE_EVENT_GUID,
    0x6f1d2b3a, 0x1034, 0x4a7f, 0x9e, 0x2c, 0x5a, 0x11, 0xc0, 0xff, 0x21, 0x02);

#define RVF1034_WMI_PROBE_PAYLOAD_SCHEMA       1
#define RVF1034_WMI_PROBE_KIND_CONTROL_ENABLE  1
#define RVF1034_WMI_PROBE_KIND_CONTROL_DISABLE 2
#define RVF1034_WMI_PROBE_KIND_WRITE_MIRROR    3
#define RVF1034_WMI_PROBE_CONTROL_ORDINAL      0xffffffffUL
#define RVF1034_WMI_PROBE_MIRROR_SLOTS         24
#define RVF1034_WMI_PROBE_MIRROR_FIRST         4ULL
#define RVF1034_WMI_PROBE_MIRROR_INTERVAL      32ULL

#define RVF1034_WMI_PROBE_FLAG_HELPER_PRESENT  0x00000001
#define RVF1034_WMI_PROBE_FLAG_ENABLED         0x00000002
#define RVF1034_WMI_PROBE_FLAG_REGISTERED      0x00000004
#define RVF1034_WMI_PROBE_FLAG_LOGGER_HANDLE   0x00000008
#define RVF1034_WMI_PROBE_FLAG_ENABLE_VALUE    0x00000010

typedef struct _RVF1034_WMI_PROBE_PAYLOAD
{
    UCHAR       Signature[8];
    ULONG       SchemaVersion;
    ULONG       Kind;
    ULONGLONG   Sequence;
    ULONG       Status;
    ULONG       Flags;
    ULONG       MiniportEventOrdinal;
    ULONG       MiniportSchemaId;
    ULONGLONG   Data1;
    ULONGLONG   Data2;
    ULONGLONG   Data3;
    ULONGLONG   Data4;
    ULONGLONG   WriteEtwEventCalls;
    ULONGLONG   HelperNullCalls;
    ULONGLONG   ForwardErrorCalls;
    ULONGLONG   MirrorSuppressedCalls;
    ULONGLONG   MirrorKeyCalls;
    ULONGLONG   MirrorKeyEmitCalls;
} RVF1034_WMI_PROBE_PAYLOAD;

typedef struct _RVF1034_WMI_PROBE_EVENT
{
    EVENT_TRACE_HEADER          Header;
    RVF1034_WMI_PROBE_PAYLOAD   Payload;
} RVF1034_WMI_PROBE_EVENT;

typedef struct _RVF1034_WMI_PROBE_CONTROL_WORK_CONTEXT
{
    ULONG       Kind;
    NTSTATUS    Status;
    ULONG       Flags;
    ULONG       GuidIndex;
    ULONG       Function;
    ULONGLONG   EnableCount;
    ULONGLONG   DisableCount;
    LONG64      LoggerHandle;
} RVF1034_WMI_PROBE_CONTROL_WORK_CONTEXT;

static WMIGUIDREGINFO g_RvfWmiProbeGuidList[] =
{
    { &RVF1034_WMI_PROBE_CONTROL_GUID, 1, WMIREG_FLAG_TRACED_GUID | WMIREG_FLAG_TRACE_CONTROL_GUID },
    { &RVF1034_WMI_PROBE_EVENT_GUID, 1, WMIREG_FLAG_TRACED_GUID }
};

static WMILIB_CONTEXT g_RvfWmiProbeContext;
static PPORTWMIREGISTRATION g_RvfWmiProbeRegistration;
static LONG64 volatile g_RvfWmiProbeLoggerHandle;
static LONG volatile g_RvfWmiProbeEnabled;
static LONG volatile g_RvfWmiProbeRegistered;
static LONG volatile g_RvfWmiProbeRegistrationStatus;
static LONG volatile g_RvfWmiProbeUnregisterStatus;
static LONG volatile g_RvfWmiProbeLastFireStatus;
static LONG64 volatile g_RvfWmiProbeEnableCount;
static LONG64 volatile g_RvfWmiProbeDisableCount;
static LONG64 volatile g_RvfWmiProbeSequence;
static LONG64 volatile g_RvfWriteEtwEventCalls;
static LONG64 volatile g_RvfWriteEtwEventHelperNullCalls;
static LONG64 volatile g_RvfWriteEtwEventForwardErrorCalls;
static LONG64 volatile g_RvfWriteEtwEventMirrorSuppressedCalls;
static LONG64 volatile g_RvfWriteEtwEventMirrorKeyCalls[RVF1034_WMI_PROBE_MIRROR_SLOTS];
static LONG64 volatile g_RvfWriteEtwEventMirrorKeyEmitCalls[RVF1034_WMI_PROBE_MIRROR_SLOTS];

static
ULONG
RvfWmiProbeMirrorSlot
(
    _In_ EPcMiniportEngineEvent MiniportEventType,
    _In_ ULONGLONG Data1
)
{
    ULONG ordinal = (ULONG)MiniportEventType;

    if (ordinal == 0)
    {
        ULONG schemaId = (ULONG)(Data1 & 0xffff);
        if (schemaId >= 0x1001 && schemaId <= 0x1005)
        {
            return 16 + (schemaId - 0x1001);
        }
        if (schemaId == 0x2001)
        {
            return 21;
        }
        return 0;
    }

    if (ordinal < 16)
    {
        return ordinal;
    }
    return 23;
}

static
NTSTATUS
RvfWmiProbeFireEventWithLogger
(
    _In_ LONG64 LoggerHandle,
    _In_ ULONG Kind,
    _In_ NTSTATUS Status,
    _In_ ULONG Flags,
    _In_ ULONG MiniportEventOrdinal,
    _In_ ULONG MiniportSchemaId,
    _In_ ULONGLONG Data1,
    _In_ ULONGLONG Data2,
    _In_ ULONGLONG Data3,
    _In_ ULONGLONG Data4,
    _In_ ULONGLONG MirrorKeyCalls,
    _In_ ULONGLONG MirrorKeyEmitCalls
)
{
    LONG64 loggerHandle = LoggerHandle;
    RVF1034_WMI_PROBE_EVENT event;
    NTSTATUS fireStatus;

    if (loggerHandle == 0)
    {
        return STATUS_INVALID_HANDLE;
    }

    RtlZeroMemory(&event, sizeof(event));
    event.Header.Size = (USHORT)sizeof(event);
    event.Header.Class.Type = (UCHAR)Kind;
    event.Header.Class.Level = 5;
    event.Header.Class.Version = 1;
    event.Header.Flags = WNODE_FLAG_TRACED_GUID;
    event.Header.Guid = RVF1034_WMI_PROBE_EVENT_GUID;

    event.Payload.Signature[0] = 'R';
    event.Payload.Signature[1] = 'V';
    event.Payload.Signature[2] = 'F';
    event.Payload.Signature[3] = '1';
    event.Payload.Signature[4] = '0';
    event.Payload.Signature[5] = '3';
    event.Payload.Signature[6] = '4';
    event.Payload.Signature[7] = 'W';
    event.Payload.SchemaVersion = RVF1034_WMI_PROBE_PAYLOAD_SCHEMA;
    event.Payload.Kind = Kind;
    event.Payload.Sequence = (ULONGLONG)InterlockedIncrement64(&g_RvfWmiProbeSequence);
    event.Payload.Status = (ULONG)Status;
    event.Payload.Flags = Flags;
    event.Payload.MiniportEventOrdinal = MiniportEventOrdinal;
    event.Payload.MiniportSchemaId = MiniportSchemaId;
    event.Payload.Data1 = Data1;
    event.Payload.Data2 = Data2;
    event.Payload.Data3 = Data3;
    event.Payload.Data4 = Data4;
    event.Payload.WriteEtwEventCalls = (ULONGLONG)InterlockedCompareExchange64(&g_RvfWriteEtwEventCalls, 0, 0);
    event.Payload.HelperNullCalls = (ULONGLONG)InterlockedCompareExchange64(&g_RvfWriteEtwEventHelperNullCalls, 0, 0);
    event.Payload.ForwardErrorCalls = (ULONGLONG)InterlockedCompareExchange64(&g_RvfWriteEtwEventForwardErrorCalls, 0, 0);
    event.Payload.MirrorSuppressedCalls = (ULONGLONG)InterlockedCompareExchange64(&g_RvfWriteEtwEventMirrorSuppressedCalls, 0, 0);
    event.Payload.MirrorKeyCalls = MirrorKeyCalls;
    event.Payload.MirrorKeyEmitCalls = MirrorKeyEmitCalls;

    ((PWNODE_HEADER)&event.Header)->HistoricalContext = (ULONG64)loggerHandle;
    fireStatus = IoWMIWriteEvent(&event);
    InterlockedExchange(&g_RvfWmiProbeLastFireStatus, (LONG)fireStatus);
    return fireStatus;
}

static
NTSTATUS
RvfWmiProbeFireEvent
(
    _In_ ULONG Kind,
    _In_ NTSTATUS Status,
    _In_ ULONG Flags,
    _In_ ULONG MiniportEventOrdinal,
    _In_ ULONG MiniportSchemaId,
    _In_ ULONGLONG Data1,
    _In_ ULONGLONG Data2,
    _In_ ULONGLONG Data3,
    _In_ ULONGLONG Data4,
    _In_ ULONGLONG MirrorKeyCalls,
    _In_ ULONGLONG MirrorKeyEmitCalls
)
{
    return RvfWmiProbeFireEventWithLogger(
        InterlockedCompareExchange64(&g_RvfWmiProbeLoggerHandle, 0, 0),
        Kind,
        Status,
        Flags,
        MiniportEventOrdinal,
        MiniportSchemaId,
        Data1,
        Data2,
        Data3,
        Data4,
        MirrorKeyCalls,
        MirrorKeyEmitCalls);
}

static
VOID
RvfWmiProbeControlWorkItem
(
    _In_ PVOID IoObject,
    _In_opt_ PVOID Context,
    _In_ PIO_WORKITEM IoWorkItem
)
{
    RVF1034_WMI_PROBE_CONTROL_WORK_CONTEXT* workContext =
        (RVF1034_WMI_PROBE_CONTROL_WORK_CONTEXT*)Context;

    UNREFERENCED_PARAMETER(IoObject);

    if (workContext != NULL)
    {
        RvfWmiProbeFireEventWithLogger(
            workContext->LoggerHandle,
            workContext->Kind,
            workContext->Status,
            workContext->Flags,
            RVF1034_WMI_PROBE_CONTROL_ORDINAL,
            0,
            workContext->GuidIndex,
            workContext->Function,
            workContext->EnableCount,
            workContext->DisableCount,
            0,
            0);
        ExFreePoolWithTag(workContext, SYSVAD_POOLTAG);
    }

    IoFreeWorkItem(IoWorkItem);
}

static
VOID
RvfWmiProbeQueueControlEvent
(
    _In_ PDEVICE_OBJECT DeviceObject,
    _In_ ULONG Kind,
    _In_ NTSTATUS Status,
    _In_ ULONG Flags,
    _In_ ULONG GuidIndex,
    _In_ WMIENABLEDISABLECONTROL Function,
    _In_ ULONGLONG EnableCount,
    _In_ ULONGLONG DisableCount,
    _In_ LONG64 LoggerHandle
)
{
    PIO_WORKITEM workItem = IoAllocateWorkItem(DeviceObject);
    RVF1034_WMI_PROBE_CONTROL_WORK_CONTEXT* workContext;

    if (workItem == NULL)
    {
        return;
    }

    workContext = (RVF1034_WMI_PROBE_CONTROL_WORK_CONTEXT*)ExAllocatePool2(
        POOL_FLAG_NON_PAGED,
        sizeof(RVF1034_WMI_PROBE_CONTROL_WORK_CONTEXT),
        SYSVAD_POOLTAG);
    if (workContext == NULL)
    {
        IoFreeWorkItem(workItem);
        return;
    }

    workContext->Kind = Kind;
    workContext->Status = Status;
    workContext->Flags = Flags;
    workContext->GuidIndex = GuidIndex;
    workContext->Function = (ULONG)Function;
    workContext->EnableCount = EnableCount;
    workContext->DisableCount = DisableCount;
    workContext->LoggerHandle = LoggerHandle;

    IoQueueWorkItemEx(workItem, RvfWmiProbeControlWorkItem, CriticalWorkQueue, workContext);
}

static
ULONG
RvfWmiProbeStateFlags
(
    _In_ ULONG AdditionalFlags
)
{
    ULONG flags = AdditionalFlags;

    if (InterlockedCompareExchange(&g_RvfWmiProbeEnabled, 0, 0))
    {
        flags |= RVF1034_WMI_PROBE_FLAG_ENABLED;
    }
    if (InterlockedCompareExchange(&g_RvfWmiProbeRegistered, 0, 0))
    {
        flags |= RVF1034_WMI_PROBE_FLAG_REGISTERED;
    }
    if (InterlockedCompareExchange64(&g_RvfWmiProbeLoggerHandle, 0, 0) != 0)
    {
        flags |= RVF1034_WMI_PROBE_FLAG_LOGGER_HANDLE;
    }
    return flags;
}

#pragma code_seg("PAGE")
static
NTSTATUS
RvfWmiProbeQueryRegInfo
(
    _Inout_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PULONG RegFlags,
    _Inout_ PUNICODE_STRING InstanceName,
    _Outptr_result_maybenull_ PUNICODE_STRING *RegistryPath,
    _Inout_ PUNICODE_STRING MofResourceName,
    _Outptr_result_maybenull_ PDEVICE_OBJECT *Pdo
)
{
    PAGED_CODE();

    UNREFERENCED_PARAMETER(InstanceName);
    UNREFERENCED_PARAMETER(MofResourceName);

    *RegFlags = WMIREG_FLAG_INSTANCE_PDO;
    *RegistryPath = &g_RegistryPath;
    return PcGetPhysicalDeviceObject(DeviceObject, Pdo);
}

#pragma code_seg("PAGE")
static
NTSTATUS
RvfWmiProbeQueryDataBlock
(
    _Inout_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PIRP Irp,
    _In_ ULONG GuidIndex,
    _In_ ULONG InstanceIndex,
    _In_ ULONG InstanceCount,
    _Out_writes_opt_(InstanceCount) PULONG InstanceLengthArray,
    _In_ ULONG BufferAvail,
    _Out_writes_bytes_opt_(BufferAvail) PUCHAR Buffer
)
{
    PAGED_CODE();

    UNREFERENCED_PARAMETER(GuidIndex);
    UNREFERENCED_PARAMETER(InstanceIndex);
    UNREFERENCED_PARAMETER(InstanceCount);
    UNREFERENCED_PARAMETER(InstanceLengthArray);
    UNREFERENCED_PARAMETER(BufferAvail);
    UNREFERENCED_PARAMETER(Buffer);

    return WmiCompleteRequest(DeviceObject, Irp, STATUS_WMI_GUID_NOT_FOUND, 0, IO_NO_INCREMENT);
}

#pragma code_seg("PAGE")
static
NTSTATUS
RvfWmiProbeFunctionControl
(
    _Inout_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PIRP Irp,
    _In_ ULONG GuidIndex,
    _In_ WMIENABLEDISABLECONTROL Function,
    _In_ BOOLEAN Enable
)
{
    PAGED_CODE();

    if (Function == WmiEventControl && GuidIndex < ARRAYSIZE(g_RvfWmiProbeGuidList))
    {
        PIO_STACK_LOCATION stack = IoGetCurrentIrpStackLocation(Irp);
        ULONGLONG enableCount = (ULONGLONG)InterlockedCompareExchange64(&g_RvfWmiProbeEnableCount, 0, 0);
        ULONGLONG disableCount = (ULONGLONG)InterlockedCompareExchange64(&g_RvfWmiProbeDisableCount, 0, 0);
        NTSTATUS registrationStatus = (NTSTATUS)InterlockedCompareExchange(&g_RvfWmiProbeRegistrationStatus, 0, 0);
        ULONG additionalFlags = Enable ? RVF1034_WMI_PROBE_FLAG_ENABLE_VALUE : 0;

        if (Enable)
        {
            LONG64 loggerHandle = 0;

            if (stack->Parameters.WMI.BufferSize >= sizeof(WNODE_HEADER) && stack->Parameters.WMI.Buffer != NULL)
            {
                PWNODE_HEADER wnode = (PWNODE_HEADER)stack->Parameters.WMI.Buffer;
                loggerHandle = (LONG64)wnode->HistoricalContext;
            }

            InterlockedExchange64(&g_RvfWmiProbeLoggerHandle, loggerHandle);
            InterlockedExchange(&g_RvfWmiProbeEnabled, loggerHandle != 0 ? 1 : 0);
            enableCount = (ULONGLONG)InterlockedIncrement64(&g_RvfWmiProbeEnableCount);

            RvfWmiProbeQueueControlEvent(
                DeviceObject,
                RVF1034_WMI_PROBE_KIND_CONTROL_ENABLE,
                registrationStatus,
                RvfWmiProbeStateFlags(additionalFlags),
                GuidIndex,
                Function,
                enableCount,
                disableCount,
                loggerHandle);
        }
        else
        {
            disableCount = (ULONGLONG)InterlockedIncrement64(&g_RvfWmiProbeDisableCount);

            LONG64 loggerHandle = InterlockedCompareExchange64(&g_RvfWmiProbeLoggerHandle, 0, 0);

            RvfWmiProbeQueueControlEvent(
                DeviceObject,
                RVF1034_WMI_PROBE_KIND_CONTROL_DISABLE,
                registrationStatus,
                RvfWmiProbeStateFlags(additionalFlags),
                GuidIndex,
                Function,
                enableCount,
                disableCount,
                loggerHandle);

            InterlockedExchange(&g_RvfWmiProbeEnabled, 0);
            InterlockedExchange64(&g_RvfWmiProbeLoggerHandle, 0);
        }
    }

    return WmiCompleteRequest(DeviceObject, Irp, STATUS_SUCCESS, 0, IO_NO_INCREMENT);
}

static
VOID
RvfWmiProbeResetRuntimeCounters()
{
    InterlockedExchange64(&g_RvfWmiProbeLoggerHandle, 0);
    InterlockedExchange(&g_RvfWmiProbeEnabled, 0);
    InterlockedExchange64(&g_RvfWmiProbeEnableCount, 0);
    InterlockedExchange64(&g_RvfWmiProbeDisableCount, 0);
    InterlockedExchange64(&g_RvfWmiProbeSequence, 0);
    InterlockedExchange64(&g_RvfWriteEtwEventCalls, 0);
    InterlockedExchange64(&g_RvfWriteEtwEventHelperNullCalls, 0);
    InterlockedExchange64(&g_RvfWriteEtwEventForwardErrorCalls, 0);
    InterlockedExchange64(&g_RvfWriteEtwEventMirrorSuppressedCalls, 0);

    for (ULONG index = 0; index < RVF1034_WMI_PROBE_MIRROR_SLOTS; index++)
    {
        InterlockedExchange64(&g_RvfWriteEtwEventMirrorKeyCalls[index], 0);
        InterlockedExchange64(&g_RvfWriteEtwEventMirrorKeyEmitCalls[index], 0);
    }
}

#pragma code_seg("PAGE")
NTSTATUS
RvfWmiProbeRegister
(
    _In_ PDEVICE_OBJECT DeviceObject,
    _In_opt_ PPORTWMIREGISTRATION PortWmiRegistration,
    _In_ NTSTATUS QueryStatus
)
{
    NTSTATUS ntStatus = STATUS_SUCCESS;

    PAGED_CODE();

    if (PortWmiRegistration == NULL)
    {
        InterlockedExchange(&g_RvfWmiProbeRegistrationStatus, (LONG)QueryStatus);
        return STATUS_SUCCESS;
    }

    if (g_RvfWmiProbeRegistration != NULL)
    {
        return STATUS_SUCCESS;
    }

    RtlZeroMemory(&g_RvfWmiProbeContext, sizeof(g_RvfWmiProbeContext));
    g_RvfWmiProbeContext.GuidCount = ARRAYSIZE(g_RvfWmiProbeGuidList);
    g_RvfWmiProbeContext.GuidList = g_RvfWmiProbeGuidList;
    g_RvfWmiProbeContext.QueryWmiRegInfo = RvfWmiProbeQueryRegInfo;
    g_RvfWmiProbeContext.QueryWmiDataBlock = RvfWmiProbeQueryDataBlock;
    g_RvfWmiProbeContext.WmiFunctionControl = RvfWmiProbeFunctionControl;

    RvfWmiProbeResetRuntimeCounters();
    ntStatus = PortWmiRegistration->RegisterWMIProvider(DeviceObject, &g_RvfWmiProbeContext);
    InterlockedExchange(&g_RvfWmiProbeRegistrationStatus, (LONG)ntStatus);

    if (NT_SUCCESS(ntStatus))
    {
        g_RvfWmiProbeRegistration = PortWmiRegistration;
        g_RvfWmiProbeRegistration->AddRef();
        InterlockedExchange(&g_RvfWmiProbeRegistered, 1);
    }

    return ntStatus;
}

#pragma code_seg("PAGE")
VOID
RvfWmiProbeUnregister
(
    _In_ PDEVICE_OBJECT DeviceObject
)
{
    PAGED_CODE();

    InterlockedExchange(&g_RvfWmiProbeEnabled, 0);
    InterlockedExchange64(&g_RvfWmiProbeLoggerHandle, 0);

    if (g_RvfWmiProbeRegistration != NULL)
    {
        NTSTATUS ntStatus = g_RvfWmiProbeRegistration->UnregisterWMIProvider(DeviceObject);
        InterlockedExchange(&g_RvfWmiProbeUnregisterStatus, (LONG)ntStatus);
        g_RvfWmiProbeRegistration->Release();
        g_RvfWmiProbeRegistration = NULL;
    }

    InterlockedExchange(&g_RvfWmiProbeRegistered, 0);
}

#pragma code_seg()
VOID
RvfWmiProbeMirrorWriteEtwEvent
(
    _In_ EPcMiniportEngineEvent MiniportEventType,
    _In_ ULONGLONG Data1,
    _In_ ULONGLONG Data2,
    _In_ ULONGLONG Data3,
    _In_ ULONGLONG Data4,
    _In_ NTSTATUS ForwardStatus,
    _In_ BOOLEAN HelperPresent
)
{
    ULONG slot = RvfWmiProbeMirrorSlot(MiniportEventType, Data1);
    ULONGLONG keyCalls;
    ULONGLONG keyEmitCalls = 0;
    BOOLEAN shouldEmit;

    InterlockedIncrement64(&g_RvfWriteEtwEventCalls);
    if (!HelperPresent)
    {
        InterlockedIncrement64(&g_RvfWriteEtwEventHelperNullCalls);
    }
    else if (!NT_SUCCESS(ForwardStatus))
    {
        InterlockedIncrement64(&g_RvfWriteEtwEventForwardErrorCalls);
    }

    keyCalls = (ULONGLONG)InterlockedIncrement64(&g_RvfWriteEtwEventMirrorKeyCalls[slot]);
    shouldEmit = keyCalls <= RVF1034_WMI_PROBE_MIRROR_FIRST ||
        (keyCalls % RVF1034_WMI_PROBE_MIRROR_INTERVAL) == 0;

    if (!shouldEmit)
    {
        InterlockedIncrement64(&g_RvfWriteEtwEventMirrorSuppressedCalls);
        return;
    }

    if (!InterlockedCompareExchange(&g_RvfWmiProbeEnabled, 0, 0))
    {
        return;
    }

    keyEmitCalls = (ULONGLONG)InterlockedIncrement64(&g_RvfWriteEtwEventMirrorKeyEmitCalls[slot]);
    RvfWmiProbeFireEvent(
        RVF1034_WMI_PROBE_KIND_WRITE_MIRROR,
        ForwardStatus,
        RvfWmiProbeStateFlags(HelperPresent ? RVF1034_WMI_PROBE_FLAG_HELPER_PRESENT : 0),
        (ULONG)MiniportEventType,
        (ULONG)(Data1 & 0xffff),
        Data1,
        Data2,
        Data3,
        Data4,
        keyCalls,
        keyEmitCalls);
}
