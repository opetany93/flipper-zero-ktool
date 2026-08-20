//! Serial ports.
//!
//! `SerialHandle` from the `flipperzero` crate already covers acquiring,
//! configuring and transmitting. The one thing it does not expose is the frame
//! format, and that is what this module adds.

use flipperzero::serial;
use flipperzero::serial::SerialHandle;
use flipperzero_sys as sys;

/// The two serial peripherals broken out on the GPIO header: USART1 on pins
/// 13/14, LPUART1 on 15/16.
#[derive(Clone, Copy, Debug)]
pub enum Port {
    Usart,
    Lpuart,
}

/// The shape of one character on the wire: data bits, parity, stop bits.
///
/// Whole named profiles rather than three separate knobs, which keeps the `sys`
/// enums inside `hal` and makes a call site read as one decision instead of
/// three.
#[derive(Clone, Copy, Debug)]
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
}

/// An open serial port.
pub struct SerialPort {
    handle: SerialHandle,
}

/// The port is already taken, by the log device or by the Expansion Modules
/// service. Both are Flipper settings rather than faults, so this is worth
/// showing to the user rather than panicking on.
#[derive(Debug)]
pub struct PortBusy;

impl SerialPort {
    /// Acquires `port`, brings it up at `baud` and applies `framing`.
    ///
    /// The order is not free to change: `init` configures the peripheral with
    /// 8N1, so framing has to be applied after it rather than before.
    pub fn open(port: Port, baud: u32, framing: Framing) -> Result<Self, PortBusy> {
        let id = match port {
            Port::Usart => serial::USART,
            Port::Lpuart => serial::LPUART,
        };

        let handle = SerialHandle::acquire(id).map_err(|_| PortBusy)?;

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

        Ok(Self { handle })
    }

    // pub fn write(&mut self, data: &[u8]) {}

    // pub fn read(&mut self, buffer: &mut [u8]) -> usize {}
}
