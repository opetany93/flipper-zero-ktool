//! The application thread: sampling, shutdown ordering, and the state the GUI
//! thread reads.

use core::fmt::Write;

use flipperzero::error;
use flipperzero::furi::sync::Mutex;
use flipperzero::furi::time::FuriDuration;
use flipperzero::info;

use crate::event::{Event, EventQueue};
use crate::hal::canvas::Canvas;
use crate::hal::input::{InputEvent, Key, Press};
use crate::hal::serial::{Port, SerialPort};
use crate::hal::timer::PeriodicTimer;
use crate::hal::view_port::ViewPort;
use crate::supply::{Reading, VoltageSource};
use crate::text::TextBuffer;
use crate::ui;

/// Sampling period, and so the screen refresh rate.
const SAMPLE_PERIOD_MS: u64 = 500;

/// Bytes taken from a port per notification.
const RECEIVE_CHUNK: usize = 16;

/// Room for `RECEIVE_CHUNK` of `"XX "`, terminator included.
const HEX_DUMP_CAPACITY: usize = 3 * RECEIVE_CHUNK + 1;

/// Runs KTool until the user presses Back.
pub fn run(
    supply: &mut impl VoltageSource,
    kline_serial_port: Option<SerialPort<'_>>,
    kbus_serial_port: Option<SerialPort<'_>>,
) {
    // The only two things other threads reach into. Sampling stays outside the
    // mutex: an ADC conversion is far too long to hold a lock the GUI thread
    // needs in order to draw.
    let events_queue = EventQueue::new();
    let reading = Mutex::new(Reading::default());

    // So the first frame is not blank.
    let sample = supply.read();
    *reading.lock() = sample;

    // Runs in the receive interrupt, so it must only hand the loop a nudge and
    // return. Reading the bytes is the loop's job.
    let on_serial_data = |port| events_queue.try_post(Event::SerialData(port));

    let Some(mut kline_serial_port) = kline_serial_port else {
        error!("K-Line serial port not available");
        return;
    };

    let Some(mut kbus_serial_port) = kbus_serial_port else {
        error!("K-Bus serial port not available");
        return;
    };

    kline_serial_port.set_on_data(&on_serial_data);
    kbus_serial_port.set_on_data(&on_serial_data);

    kline_serial_port.transmit(&[0x55, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    kbus_serial_port.transmit(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);

    let on_draw = draw_callback(&reading);
    let on_input = |event: InputEvent| events_queue.post(Event::Input(event));
    let on_tick = || events_queue.try_post(Event::Tick);

    // Declaration order is shutdown order reversed, and the borrow checker
    // enforces it.
    let view_port = ViewPort::fullscreen(&on_draw, &on_input);
    let mut timer = PeriodicTimer::new(&on_tick);

    timer.start(FuriDuration::from_millis(SAMPLE_PERIOD_MS));

    while let Some(event) = events_queue.next() {
        match event {
            Event::Input(InputEvent {
                key: Key::Back,
                press: Press::Short,
            }) => break,
            Event::Input(_) => {}
            Event::Tick => {
                let sample = supply.read();
                *reading.lock() = sample;

                view_port.request_redraw();
            }
            Event::SerialData(Port::Usart) => log_received("K-Line", &kline_serial_port),
            Event::SerialData(Port::Lpuart) => log_received("K-Bus", &kbus_serial_port),
        }
    }
}

/// Builds the drawing callback. It runs on the GUI service thread, which is
/// why it may only read `reading` and must never block.
fn draw_callback(reading: &Mutex<Reading>) -> impl Fn(&mut Canvas<'_>) + Sync + '_ {
    move |canvas| {
        // Snapshot and release: the lock is never held across drawing.
        let snapshot = *reading.lock();

        ui::draw(canvas, &snapshot, true, true);
    }
}

/// Scaffolding for the echo test, until there is a screen to show bytes on.
fn log_received(bus: &str, serial_port: &SerialPort<'_>) {
    let mut received = [0u8; RECEIVE_CHUNK];
    let count = serial_port.read(&mut received);
    if 0 == count {
        return;
    }

    let mut hex = TextBuffer::<HEX_DUMP_CAPACITY>::new();
    for byte in &received[..count] {
        let _ = write!(hex, "{:02X} ", byte);
    }

    info!("{} received {} bytes: {}", bus, count, hex.as_str());
}
