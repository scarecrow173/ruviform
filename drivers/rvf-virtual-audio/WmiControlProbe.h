/*++

Copyright (c) Microsoft Corporation All Rights Reserved

Module Name:

    WmiControlProbe.h

Abstract:

    rf#1034 diagnostic-only driver-owned WMI control/callsite probe.

--*/

#ifndef _RVF_WMI_CONTROL_PROBE_H_
#define _RVF_WMI_CONTROL_PROBE_H_

#include <portcls.h>

NTSTATUS
RvfWmiProbeRegister
(
    _In_ PDEVICE_OBJECT DeviceObject,
    _In_opt_ PPORTWMIREGISTRATION PortWmiRegistration,
    _In_ NTSTATUS QueryStatus
);

VOID
RvfWmiProbeUnregister
(
    _In_ PDEVICE_OBJECT DeviceObject
);

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
);

#endif // _RVF_WMI_CONTROL_PROBE_H_
