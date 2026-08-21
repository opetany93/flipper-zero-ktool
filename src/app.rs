//! The application thread: sampling, shutdown ordering, and the state the GUI
//! thread reads.

use flipperzero::error;
use flipperzero::furi::sync::Mutex;
use flipperzero::furi::time::FuriDuration;
use flipperzero::info;

use crate::event::{Event, EventQueue};
use crate::hal::canvas::Canvas;
use crate::hal::input::{InputEvent, Key, Press};
use crate::hal::serial::SerialPort;
use crate::hal::timer::PeriodicTimer;
use crate::hal::view_port::ViewPort;
use crate::supply::{Reading, VoltageSource};
use crate::ui;

/// Sampling period, and so the screen refresh rate.
const SAMPLE_PERIOD_MS: u64 = 500;

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
    let on_serial_data = || events_queue.try_post(Event::SerialData);

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

    kline_serial_port.transmit(&[0x55]);
    kbus_serial_port.transmit(&[0xAA]);

    let on_draw = |canvas: &mut Canvas<'_>| {
        // Snapshot and release: the lock is never held across drawing.
        let snapshot = *reading.lock();

        ui::draw(canvas, &snapshot, true, true);
    };
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
            Event::SerialData => {
                log_received("K-Line", &kline_serial_port);
                log_received("K-Bus", &kbus_serial_port);
            }
        }
    }
}

/// Scaffolding for the echo test, until there is a screen to show bytes on.
fn log_received(bus: &str, serial_port: &SerialPort<'_>) {
    let mut received = [0u8; 16];
    let count = serial_port.read(&mut received);
    if 0 == count {
        return;
    }

    info!(
        "{} received {} bytes, first 0x{:X}",
        bus, count, received[0]
    );
}
