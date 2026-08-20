//! The application thread: sampling, shutdown ordering, and the state the GUI
//! thread reads.

use flipperzero::furi::sync::Mutex;
use flipperzero::furi::time::FuriDuration;

use crate::event::{Event, EventQueue};
use crate::hal::canvas::Canvas;
use crate::hal::input::{InputEvent, Key, Press};
use crate::hal::timer::PeriodicTimer;
use crate::hal::view_port::ViewPort;
use crate::sensor::{SupplyReading, SupplyVoltageSource};
use crate::ui;

/// Sampling period, and so the screen refresh rate.
const SAMPLE_PERIOD_MS: u64 = 500;

/// Runs KTool until the user presses Back.
pub fn run(supply: &mut impl SupplyVoltageSource) {
    // The only two things other threads reach into. Sampling stays outside the
    // mutex: an ADC conversion is far too long to hold a lock the GUI thread
    // needs in order to draw.
    let events_queue = EventQueue::new();
    let reading = Mutex::new(SupplyReading::default());

    // So the first frame is not blank.
    *reading.lock() = supply.read();

    let on_draw = |canvas: &mut Canvas<'_>| {
        // Snapshot and release: the lock is never held across drawing.
        let snapshot = *reading.lock();

        ui::draw(canvas, &snapshot);
    };
    let on_input = |event: InputEvent| events_queue.post(Event::Input(event));
    let on_tick = || events_queue.try_post(Event::Tick);

    // Declaration order is shutdown order reversed: the timer stops and the view
    // port detaches before the closures they call, and before the queue and the
    // mutex those captured. The borrow checker rejects any other ordering.
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
                *reading.lock() = supply.read();
                view_port.request_redraw();
            }
        }
    }
}
