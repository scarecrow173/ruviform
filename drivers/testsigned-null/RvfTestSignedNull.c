// RvfTestSignedNull.c — Issue #961 install 可否判定用の最小 WDM null driver。
// root-enumerated software device（hardware id `ROOT\RvfTestSigned`）の
// function driver として load されることだけが目的。audio/MIDI 機能は
// 持たず、KMDF にも依存しない（WDM のみ — INF から coinstaller を除外し、
// test-signing の成立可否だけを測る最小構成）。
#include <wdm.h>

DRIVER_UNLOAD RvfnUnload;
DRIVER_ADD_DEVICE RvfnAddDevice;
DRIVER_DISPATCH RvfnDispatchPnp;
DRIVER_DISPATCH RvfnDispatchPower;
DRIVER_DISPATCH RvfnDispatchDefault;

typedef struct _RVFN_DEVICE_EXTENSION {
    PDEVICE_OBJECT LowerDevice;
} RVFN_DEVICE_EXTENSION, *PRVFN_DEVICE_EXTENSION;

NTSTATUS
DriverEntry(
    _In_ PDRIVER_OBJECT DriverObject,
    _In_ PUNICODE_STRING RegistryPath
    )
{
    ULONG i;

    UNREFERENCED_PARAMETER(RegistryPath);

    DriverObject->DriverExtension->AddDevice = RvfnAddDevice;
    DriverObject->DriverUnload = RvfnUnload;
    DriverObject->MajorFunction[IRP_MJ_PNP] = RvfnDispatchPnp;
    DriverObject->MajorFunction[IRP_MJ_POWER] = RvfnDispatchPower;
    for (i = 0; i <= IRP_MJ_MAXIMUM_FUNCTION; i++) {
        if (DriverObject->MajorFunction[i] == NULL) {
            DriverObject->MajorFunction[i] = RvfnDispatchDefault;
        }
    }
    return STATUS_SUCCESS;
}

VOID
RvfnUnload(
    _In_ PDRIVER_OBJECT DriverObject
    )
{
    UNREFERENCED_PARAMETER(DriverObject);
}

NTSTATUS
RvfnAddDevice(
    _In_ PDRIVER_OBJECT DriverObject,
    _In_ PDEVICE_OBJECT PhysicalDeviceObject
    )
{
    PDEVICE_OBJECT fdo = NULL;
    PRVFN_DEVICE_EXTENSION dx;
    NTSTATUS status;

    status = IoCreateDevice(
        DriverObject,
        sizeof(RVFN_DEVICE_EXTENSION),
        NULL,
        FILE_DEVICE_UNKNOWN,
        FILE_DEVICE_SECURE_OPEN,
        FALSE,
        &fdo);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    dx = (PRVFN_DEVICE_EXTENSION)fdo->DeviceExtension;
    dx->LowerDevice = IoAttachDeviceToDeviceStack(fdo, PhysicalDeviceObject);
    if (dx->LowerDevice == NULL) {
        IoDeleteDevice(fdo);
        return STATUS_NO_SUCH_DEVICE;
    }

    fdo->Flags |= DO_BUFFERED_IO | DO_POWER_PAGABLE;
    fdo->Flags &= ~DO_DEVICE_INITIALIZING;
    return STATUS_SUCCESS;
}

NTSTATUS
RvfnDispatchPnp(
    _In_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PIRP Irp
    )
{
    PRVFN_DEVICE_EXTENSION dx;
    PIO_STACK_LOCATION stack;
    NTSTATUS status;

    dx = (PRVFN_DEVICE_EXTENSION)DeviceObject->DeviceExtension;
    stack = IoGetCurrentIrpStackLocation(Irp);

    if (stack->MinorFunction == IRP_MN_REMOVE_DEVICE) {
        IoSkipCurrentIrpStackLocation(Irp);
        status = IoCallDriver(dx->LowerDevice, Irp);
        IoDetachDevice(dx->LowerDevice);
        IoDeleteDevice(DeviceObject);
        return status;
    }

    IoSkipCurrentIrpStackLocation(Irp);
    return IoCallDriver(dx->LowerDevice, Irp);
}

NTSTATUS
RvfnDispatchPower(
    _In_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PIRP Irp
    )
{
    PRVFN_DEVICE_EXTENSION dx;

    dx = (PRVFN_DEVICE_EXTENSION)DeviceObject->DeviceExtension;
    PoStartNextPowerIrp(Irp);
    IoSkipCurrentIrpStackLocation(Irp);
    return PoCallDriver(dx->LowerDevice, Irp);
}

NTSTATUS
RvfnDispatchDefault(
    _In_ PDEVICE_OBJECT DeviceObject,
    _Inout_ PIRP Irp
    )
{
    UNREFERENCED_PARAMETER(DeviceObject);

    Irp->IoStatus.Status = STATUS_SUCCESS;
    Irp->IoStatus.Information = 0;
    IoCompleteRequest(Irp, IO_NO_INCREMENT);
    return STATUS_SUCCESS;
}
