//! The application thread: sampling, shutdown ordering, and the state the GUI
//! thread reads.

use flipperzero::furi::sync::Mutex;
use flipperzero::furi::time::FuriDuration;

use crate::event::{Event, EventQueue};
use crate::hal::canvas::Canvas;
use crate::hal::input::{InputEvent, Key, Press};
use crate::hal::timer::PeriodicTimer;
use crate::hal::view_port::{View, ViewPort};
use crate::sensor::{SupplyReading, SupplyVoltageSource};
use crate::ui;

/// How often the supply is sampled, and so how often the screen refreshes.
const SAMPLE_PERIOD_MS: u64 = 500;

/// The state the application thread writes and the GUI thread reads.
///
/// The mutex covers the published reading and nothing else. Sampling happens
/// outside it on purpose: an ADC conversion takes far too long to hold a lock
/// the GUI thread needs in order to draw.
struct Screen {
    events: EventQueue,
    reading: Mutex<SupplyReading>,
}

impl Screen {
    fn new() -> Self {
        Self {
            events: EventQueue::new(),
            reading: Mutex::new(SupplyReading::default()),
        }
    }

    /// Makes a new reading visible to the next frame.
    fn publish(&self, reading: SupplyReading) {
        *self.reading.lock() = reading;
    }
}

impl View for Screen {
    fn draw(&self, canvas: &mut Canvas<'_>) {
        // Snapshot and release. The lock is never held across drawing.
        let reading = *self.reading.lock();

        ui::draw(canvas, &reading);
    }

    fn on_input(&self, event: InputEvent) {
        self.events.post(Event::Input(event));
    }
}

/// Runs KTool until the user presses Back.
pub fn run(supply: &mut impl SupplyVoltageSource) {
    let screen = Screen::new();

    // One reading before anything is on screen, so the first frame is not blank.
    screen.publish(supply.read());

    // Declaration order is shutdown order reversed, and that is the point: the
    // timer stops first, then the view port detaches, and only then does
    // `screen` - which both of them post into - go away. In the C version this
    // was a comment that had to be obeyed by hand; here the borrow checker
    // rejects any other ordering.
    let view_port = ViewPort::fullscreen(&screen);
    let on_tick = || screen.events.try_post(Event::Tick);
    let mut timer = PeriodicTimer::new(&on_tick);

    timer.start(FuriDuration::from_millis(SAMPLE_PERIOD_MS));

    while let Some(event) = screen.events.next() {
        match event {
            Event::Input(InputEvent {
                key: Key::Back,
                press: Press::Short,
            }) => break,
            Event::Input(_) => {}
            Event::Tick => {
                screen.publish(supply.read());
                view_port.request_redraw();
            }
        }
    }
}
