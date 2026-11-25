/*****************************************************************************
 * RvfMidiMpu.cpp - Ruviform virtual MIDI loopback byte path
 *****************************************************************************
 * Copyright (c) 1998-2000 Microsoft Corporation.  All rights reserved.
 *
 * ruviform 派生版 — WDK 8.1 "DirectMusic UART Driver Sample" (DMusUART) の
 * MPU.cpp 由来。upstream はここで実 MPU-401 UART register I/O
 * （READ_PORT_UCHAR/WRITE_PORT_UCHAR、ISR、InterruptSync）を行うが、
 * virtual device には hardware が無いため、render stream の Write() が
 * byte を内部 capture FIFO へ積み IPortDMus::Notify() で capture stream
 * の SourceEvtsToPort を駆動する software loopback に置き換えた
 * （出所・変更点は PROVENANCE.md 参照）。
 */

#pragma warning (disable : 4127)

#include "RvfMidi.h"
#include "ksdebug.h"

#define STR_MODULENAME "RvfMidi:Mpu: "

#pragma code_seg()
/*****************************************************************************
 * CMiniportRvfMidi::FeedCaptureFifo()
 *****************************************************************************
 * Loopback entry point — render stream の Write() から呼ばれ、byte 列を
 * 内部 capture FIFO へ積む。FIFO に新規 byte が入れば
 * IPortDMus::Notify(m_pServiceGroup) で capture stream の
 * SourceEvtsToPort（PutMessage(NULL) 経路）を駆動する。
 *
 * upstream の DMusMPUInterruptServiceRoutine が行っていた
 * 「UART RX FIFO → 内部 FIFO → Notify」の前半をソフトウェア化したもの。
 * capture stream が未作成/未 RUN の場合は「cable 未接続」として全 byte を
 * 消費扱いにする（render 側の ConsumeEvents を詰まらせない）。
 * FIFO が満杯のときは partial write を返し、呼出元の timer retry に任せる。
 *
 * Called at DISPATCH_LEVEL（render stream の ConsumeEvents 配下）。
 */
NTSTATUS
CMiniportRvfMidi::
FeedCaptureFifo
(
    IN      PUCHAR      Data,
    IN      ULONG       Length,
    OUT     PULONG      BytesWritten
)
{
    ASSERT(KeGetCurrentIrql() == DISPATCH_LEVEL);
    ASSERT(BytesWritten);

    *BytesWritten = 0;
    if (!Data || !Length)
    {
        return STATUS_SUCCESS;
    }

    if ((m_KSStateInput != KSSTATE_RUN) || !m_NumCaptureStreams)
    {
        // capture 側が聴いていない — 実 cable と同様に byte は捨てる。
        *BytesWritten = Length;
        return STATUS_SUCCESS;
    }

    BOOLEAN newBytesAvailable = FALSE;

    KeAcquireSpinLockAtDpcLevel(&m_InputBufferLock);
    while (*BytesWritten < Length)
    {
        if  (   (m_MidiInputBufferTail + 1 == m_MidiInputBufferHead)
            ||  (m_MidiInputBufferTail + 1 - kMidiInputBufferSize == m_MidiInputBufferHead)
            )
        {
            // FIFO full — partial write; ConsumeEvents が timer で再送する。
            break;
        }

        if (!m_InputTimeStamp && m_MasterClock)
        {
            NTSTATUS clockStatus = m_MasterClock->GetTime(&m_InputTimeStamp);
            if (STATUS_SUCCESS != clockStatus)
            {
                _DbgPrintF(DEBUGLVL_TERSE,("GetTime failed for clock 0x%08p",m_MasterClock));
            }
        }

        m_MidiInputBuffer[m_MidiInputBufferTail] = Data[*BytesWritten];
        m_MidiInputBufferTail++;
        if (m_MidiInputBufferTail >= kMidiInputBufferSize)
        {
            m_MidiInputBufferTail = 0;
        }
        (*BytesWritten)++;
        newBytesAvailable = TRUE;
    }
    KeReleaseSpinLockFromDpcLevel(&m_InputBufferLock);

    if (newBytesAvailable && m_pPort && m_pServiceGroup)
    {
        //
        // ...notify the port driver that we have bytes.  Notify は lock を
        // 解放してから呼ぶ — capture stream の SourceEvtsToPort が同じ
        // m_InputBufferLock を取りに来る経路があるため（synchronous service
        // routine 経路で再入し得る）。
        //
        m_pPort->Notify(m_pServiceGroup);
    }

    return STATUS_SUCCESS;
}

#pragma code_seg()
/*****************************************************************************
 * CMiniportRvfMidiStream::Write()
 *****************************************************************************
 * Writes outgoing MIDI data.
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidiStream::
Write
(
    IN      PVOID       BufferAddress,
    IN      ULONG       Length,
    OUT     PULONG      BytesWritten
)
{
    _DbgPrintF(DEBUGLVL_BLAB, ("Write"));
    ASSERT(BytesWritten);
    *BytesWritten = 0;

    if (!BufferAddress)
    {
        return STATUS_SUCCESS;
    }

    if (m_fCapture)
    {
        // called write on the read stream
        return STATUS_INVALID_DEVICE_REQUEST;
    }

    // render data は loopback FIFO へ流す（upstream では MPU へ write）。
    return m_pMiniport->FeedCaptureFifo(PUCHAR(BufferAddress), Length, BytesWritten);
}

#pragma code_seg()
/*****************************************************************************
 * CMiniportRvfMidiStream::SourceEvtsToPort()
 *****************************************************************************
 *
 * Reads incoming MIDI data, feeds into DMus events.
 * No need to touch the hardware, just read from our SW FIFO.
 * Notify() 経由で port の service routine から呼ばれる（DISPATCH_LEVEL）。
 */
STDMETHODIMP_(NTSTATUS)
CMiniportRvfMidiStream::SourceEvtsToPort()
{
    NTSTATUS    ntStatus;

    ASSERT(KeGetCurrentIrql() == DISPATCH_LEVEL);
    _DbgPrintF(DEBUGLVL_BLAB, ("SourceEvtsToPort"));

    if (m_fCapture)
    {
        ntStatus = STATUS_SUCCESS;
        if (m_pMiniport->m_MidiInputBufferHead != m_pMiniport->m_MidiInputBufferTail)
        {
            PDMUS_KERNEL_EVENT  aDMKEvt,eventTail,eventHead = NULL;

            while (m_pMiniport->m_MidiInputBufferHead != m_pMiniport->m_MidiInputBufferTail)
            {
                (void) m_AllocatorMXF->GetMessage(&aDMKEvt);
                if (!aDMKEvt)
                {
                    _DbgPrintF(DEBUGLVL_TERSE, ("SourceEvtsToPort can't allocate DMKEvt"));
                    return STATUS_INSUFFICIENT_RESOURCES;
                }

                //  put this event at the end of the list
                if (!eventHead)
                {
                    eventHead = aDMKEvt;
                }
                else
                {
                    eventTail = eventHead;
                    while (eventTail->pNextEvt)
                    {
                        eventTail = eventTail->pNextEvt;
                    }
                    eventTail->pNextEvt = aDMKEvt;
                }
                //  read all the bytes out of the buffer, into event(s)
                KeAcquireSpinLockAtDpcLevel(&m_pMiniport->m_InputBufferLock);
                for (aDMKEvt->cbEvent = 0; aDMKEvt->cbEvent < sizeof(PBYTE); aDMKEvt->cbEvent++)
                {
                    if (m_pMiniport->m_MidiInputBufferHead == m_pMiniport->m_MidiInputBufferTail)
                    {
                        break;
                    }
                    aDMKEvt->uData.abData[aDMKEvt->cbEvent] = m_pMiniport->m_MidiInputBuffer[m_pMiniport->m_MidiInputBufferHead];
                    m_pMiniport->m_MidiInputBufferHead++;
                    if (m_pMiniport->m_MidiInputBufferHead >= kMidiInputBufferSize)
                    {
                        m_pMiniport->m_MidiInputBufferHead = 0;
                    }
                }
                KeReleaseSpinLockFromDpcLevel(&m_pMiniport->m_InputBufferLock);
            }

            //  cache the timestamp — FIFO 先頭 byte の時刻を snapshot し、
            //  FIFO が空なら次の burst で再 stamp するよう 0 に戻す。
            KeAcquireSpinLockAtDpcLevel(&m_pMiniport->m_InputBufferLock);
            m_SnapshotTimeStamp = m_pMiniport->m_InputTimeStamp;
            if (m_pMiniport->m_MidiInputBufferHead ==
                m_pMiniport->m_MidiInputBufferTail)
            {
                m_pMiniport->m_InputTimeStamp = 0;
            }
            KeReleaseSpinLockFromDpcLevel(&m_pMiniport->m_InputBufferLock);

            aDMKEvt = eventHead;
            while (aDMKEvt)
            {
                aDMKEvt->ullPresTime100ns = m_SnapshotTimeStamp;
                aDMKEvt->usChannelGroup = 1;
                aDMKEvt->usFlags = DMUS_KEF_EVENT_INCOMPLETE;
                aDMKEvt = aDMKEvt->pNextEvt;
            }
            (void)m_sinkMXF->PutMessage(eventHead);
        }
    }
    else    //  render stream
    {
        _DbgPrintF(DEBUGLVL_TERSE, ("SourceEvtsToPort called on render stream"));
        ntStatus = STATUS_INVALID_DEVICE_REQUEST;
    }
    return ntStatus;
}
