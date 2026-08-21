//! Serial ports.
//!
//! `SerialHandle` from the `flipperzero` crate already covers acquiring,
//! configuring and transmitting. What this module adds is the frame format and
//! a receive path that works on both ports at once.

use core::ffi::c_void;
use core::marker::PhantomData;
use core::num::NonZeroUsize;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use alloc::boxed::Box;
use flipperzero::furi::stream_buffer::StreamBuffer;
use flipperzero::furi::time::FuriDuration;
use flipperzero::info;
use flipperzero::serial;
use flipperzero::serial::SerialHandle;
use flipperzero_sys as sys;

/// How much the interrupt may buffer before [`SerialPort::read`] collects it.
/// At 10400 baud that is a quarter of a second of solid traffic.
const RX_BUFFER_CAPACITY: NonZeroUsize = NonZeroUsize::new(256).unwrap();

/// Hand a byte on as soon as it lands. Nothing here blocks on the buffer, so
/// anything higher would only add latency.
const RX_TRIGGER_LEVEL: usize = 1;

static USART_RX_BUFFER: AtomicPtr<StreamBuffer> = AtomicPtr::new(ptr::null_mut());
static LPUART_RX_BUFFER: AtomicPtr<StreamBuffer> = AtomicPtr::new(ptr::null_mut());

/// The two serial peripherals broken out on the GPIO header: USART1 on pins
/// 13/14, LPUART1 on 15/16.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Port {
    Usart,
    Lpuart,
}

impl Port {
    pub fn name(self) -> &'static str {
        match self {
            Self::Usart => "USART1",
            Self::Lpuart => "LPUART1",
        }
    }

    fn id(self) -> serial::SerialId {
        match self {
            Self::Usart => serial::USART,
            Self::Lpuart => serial::LPUART,
        }
    }

    fn rx_buffer(self) -> &'static AtomicPtr<StreamBuffer> {
        match self {
            Self::Usart => &USART_RX_BUFFER,
            Self::Lpuart => &LPUART_RX_BUFFER,
        }
    }

    fn rx_callback<F: Fn(Port)>(self) -> sys::FuriHalSerialAsyncRxCallback {
        match self {
            Self::Usart => Some(usart_rx::<F>),
            Self::Lpuart => Some(lpuart_rx::<F>),
        }
    }
}

/// The shape of one character on the wire: data bits, parity, stop bits.
///
/// Whole named profiles rather than three separate knobs, which keeps the `sys`
/// enums inside `hal` and makes a call site read as one decision instead of
/// three.
#[derive(Clone, Copy, PartialEq)]
pub struct Framing {
    data_bits: sys::FuriHalSerialDataBits,
    parity: sys::FuriHalSerialParity,
    stop_bits: sys::FuriHalSerialStopBits,
}

impl Framing {
    /// 8 data bits, no parity, 1 stop bit.
    ///
    /// Also what `init` leaves behind, so passing it changes nothing - but it
    /// puts the intent in the code rather than in a default.
    pub const EIGHT_N1: Self = Self {
        data_bits: sys::FuriHalSerialDataBits8,
        parity: sys::FuriHalSerialParityNone,
        stop_bits: sys::FuriHalSerialStopBits1,
    };

    /// 8 data bits, even parity, 1 stop bit.
    pub const EIGHT_E1: Self = Self {
        parity: sys::FuriHalSerialParityEven,
        ..Self::EIGHT_N1
    };

    pub fn name(self) -> &'static str {
        match self {
            Self::EIGHT_N1 => "8N1",
            Self::EIGHT_E1 => "8E1",
            _ => "unknown framing",
        }
    }
}

/// An open port, receiving into a buffer that [`read`](Self::read) drains.
///
/// The lifetime is the notifier's: holding its borrow is what guarantees the
/// interrupt is detached before the closure can go away.
pub struct SerialPort<'a> {
    handle: SerialHandle,
    port: Port,
    _on_data: PhantomData<&'a ()>,
}

/// The port is already taken, by the log device or by the Expansion Modules
/// service. Both are Flipper settings rather than faults, so this is worth
/// showing to the user rather than panicking on.
#[derive(Debug)]
pub struct PortBusy;

impl<'a> SerialPort<'a> {
    /// Acquires `port`, brings it up at `baud`, applies `framing` and starts
    /// receiving.
    ///
    /// The order is not free to change: `init` configures the peripheral with
    /// 8N1, so framing has to be applied after it rather than before.
    pub fn open(port: Port, baud: u32, framing: Framing) -> Result<Self, PortBusy> {
        let handle = SerialHandle::acquire(port.id()).map_err(|_| PortBusy)?;

        handle.init(baud);

        // SAFETY: the handle came from `acquire` and is valid for the whole
        // call, which only reconfigures the peripheral behind it and keeps no
        // pointer of its own.
        unsafe {
            sys::furi_hal_serial_configure_framing(
                handle.as_ptr(),
                framing.data_bits,
                framing.parity,
                framing.stop_bits,
            );
        }

        // Ownership moves to the static below, and comes back in `drop`.
        let rx_buffer = Box::into_raw(Box::new(StreamBuffer::new(
            RX_BUFFER_CAPACITY,
            RX_TRIGGER_LEVEL,
        )));
        port.rx_buffer().store(rx_buffer, Ordering::Release);

        let serial_port = Self {
            handle,
            port,
            _on_data: PhantomData,
        };

        // No notifier yet, so any type satisfies the callback: the null context
        // is what tells it there is nobody to wake.
        serial_port.start_receiving::<fn(Port)>(ptr::null_mut());

        info!(
            "Serial port {} opened at {} baud, framing {}",
            port.name(),
            baud,
            framing.name()
        );

        Ok(serial_port)
    }

    /// Calls `on_data` from interrupt context once bytes have landed in the
    /// buffer, so a loop waiting on a queue can be woken instead of polling.
    /// The argument is this port, which lets one closure serve both.
    ///
    /// `on_data` runs inside the interrupt, hence `Sync`, and must neither
    /// block nor allocate.
    pub fn set_on_data<F>(&mut self, on_data: &'a F)
    where
        F: Fn(Port) + Sync,
    {
        self.start_receiving::<F>((on_data as *const F).cast_mut().cast());
    }

    pub fn rx_buffer_size(&self) -> usize {
        let rx_buffer = self.port.rx_buffer().load(Ordering::Acquire);

        let Some(rx_buffer) = (unsafe { rx_buffer.as_ref() }) else {
            return 0;
        };

        rx_buffer.bytes_available()
    }

    fn start_receiving<F: Fn(Port)>(&self, on_data: *mut c_void) {
        // Stopping first is not optional. Starting installs an interrupt
        // handler, and `furi_hal_interrupt_set_isr_ex` aborts the app if one is
        // already in place, so registering a notifier over a running receiver
        // would take the whole Flipper down.
        //
        // SAFETY: the handle is valid for as long as `self` is, the callback
        // finds its buffer through a static rather than through `context`, and
        // `on_data` is either null or a `&F` borrowed for `'a`.
        unsafe {
            sys::furi_hal_serial_async_rx_stop(self.handle.as_ptr());
            sys::furi_hal_serial_async_rx_start(
                self.handle.as_ptr(),
                self.port.rx_callback::<F>(),
                on_data,
                true,
            );
        }
    }

    /// Moves whatever the interrupt has buffered into `buffer` and returns how
    /// many bytes that was. Never blocks.
    pub fn read(&self, buffer: &mut [u8]) -> usize {
        let rx_buffer = self.port.rx_buffer().load(Ordering::Acquire);

        // SAFETY: only `drop` clears the slot, and `&self` says it has not run.
        let Some(rx_buffer) = (unsafe { rx_buffer.as_ref() }) else {
            return 0;
        };

        // SAFETY: a stream buffer takes one reader, and this is the only one:
        // nothing else in the crate receives from the port's buffer.
        unsafe { rx_buffer.receive(buffer, FuriDuration::ZERO) }
    }

    pub fn transmit(&self, data: &[u8]) {
        self.handle.tx(data);
    }
}

impl Drop for SerialPort<'_> {
    fn drop(&mut self) {
        // SAFETY: `Drop` runs before the fields are dropped, so the handle is
        // still live. This call is also what makes the free below sound: it
        // clears the callback, after which the interrupt cannot reach the
        // buffer again.
        unsafe { sys::furi_hal_serial_async_rx_stop(self.handle.as_ptr()) };

        let rx_buffer = self
            .port
            .rx_buffer()
            .swap(ptr::null_mut(), Ordering::AcqRel);

        // SAFETY: non-null, because a `SerialPort` exists only once `open` has
        // stored it, and `drop` is the only thing that takes it back out. It
        // came from `Box::into_raw`, the swap put it out of reach of a later
        // `read`, and the interrupt that shared it is stopped.
        drop(unsafe { Box::from_raw(rx_buffer) });
    }
}

// The two callbacks below must end up at different addresses, or
// `furi_hal_serial_async_rx_start` aborts the app on its second call:
// targets/f7/furi_hal/furi_hal_serial.c:868 rejects two ports sharing a
// callback pointer. Each body passes a different `Port`, which is what keeps
// the linker from folding them into one symbol.

unsafe extern "C" fn usart_rx<F: Fn(Port)>(
    handle: *mut sys::FuriHalSerialHandle,
    event: sys::FuriHalSerialRxEvent,
    on_data: *mut c_void,
) {
    // SAFETY: forwarding the arguments the interrupt handed us.
    unsafe { buffer_received_bytes::<F>(Port::Usart, handle, event, on_data) };
}

unsafe extern "C" fn lpuart_rx<F: Fn(Port)>(
    handle: *mut sys::FuriHalSerialHandle,
    event: sys::FuriHalSerialRxEvent,
    on_data: *mut c_void,
) {
    // SAFETY: forwarding the arguments the interrupt handed us.
    unsafe { buffer_received_bytes::<F>(Port::Lpuart, handle, event, on_data) };
}

/// Runs in interrupt context: no allocation, no blocking, no logging.
///
/// # Safety
///
/// `handle` must be the one the calling callback was registered with, and this
/// must be called from that callback: the two `async_rx` functions are only
/// valid there.
unsafe fn buffer_received_bytes<F: Fn(Port)>(
    port: Port,
    handle: *mut sys::FuriHalSerialHandle,
    event: sys::FuriHalSerialRxEvent,
    on_data: *mut c_void,
) {
    // `report_errors` is on, so framing, parity, noise and overrun arrive here
    // too, as a bitmask that can carry several of them at once.
    if 0 == event.0 & sys::FuriHalSerialRxEventData.0 {
        return;
    }

    let rx_buffer = port.rx_buffer().load(Ordering::Acquire);

    // SAFETY: the pointer comes from a leaked `Box`, published before the
    // interrupt was enabled, and is never cleared.
    let Some(rx_buffer) = (unsafe { rx_buffer.as_ref() }) else {
        return;
    };

    let mut buffered = 0;

    // SAFETY: called from the callback, with that callback's handle.
    while unsafe { sys::furi_hal_serial_async_rx_available(handle) } {
        let byte = unsafe { sys::furi_hal_serial_async_rx(handle) };

        // SAFETY: the only writer, and it does not block: a full buffer drops
        // the byte rather than stalling the interrupt.
        buffered += unsafe { rx_buffer.send(&[byte], FuriDuration::ZERO) };
    }

    if 0 == buffered || on_data.is_null() {
        return;
    }

    // SAFETY: `on_data` is the `&F` handed to `set_on_data`, kept borrowed for
    // as long as the port lives, and the port stops this interrupt before it
    // gives the borrow back.
    let on_data: &F = unsafe { &*on_data.cast() };

    on_data(port);
}
