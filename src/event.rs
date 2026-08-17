//! The single channel the application thread listens on.

use flipperzero::furi::message_queue::MessageQueue;
use flipperzero::furi::time::FuriDuration;

use crate::hal::input::InputEvent;

/// Everything that can wake the event loop.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// A key was pressed, held or released.
    Input(InputEvent),
    /// The sampling timer fired.
    Tick,
}

/// Slots in the queue.
///
/// Comfortably more than the number of events ever in flight, which is what
/// lets [`EventQueue::post`] be a blocking call without ever actually blocking.
const CAPACITY: usize = 8;

/// A typed mailbox for [`Event`]s, written by the GUI and timer threads and
/// drained by the application thread.
pub struct EventQueue {
    queue: MessageQueue<Event>,
}

// SAFETY: a Furi message queue is a multi-producer object by design - being
// written from other threads is its entire purpose - and every method below
// goes through `&self`. `MessageQueue` opts out of `Sync` only because it holds
// a raw pointer, not because the underlying queue is thread-hostile.
unsafe impl Sync for EventQueue {}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            queue: MessageQueue::new(CAPACITY),
        }
    }

    /// Posts an event, waiting for a slot if the queue is momentarily full.
    ///
    /// For the GUI thread, where input must not be lost. With [`CAPACITY`]
    /// slots against a handful of in-flight events, the wait never actually
    /// happens.
    pub fn post(&self, event: Event) {
        let _ = self.queue.put(event, FuriDuration::WAIT_FOREVER);
    }

    /// Posts an event, dropping it if the queue is full.
    ///
    /// For the timer service thread, where blocking would stall every timer in
    /// the system. A dropped tick costs one frame, which is a far better trade.
    pub fn try_post(&self, event: Event) {
        let _ = self.queue.put(event, FuriDuration::ZERO);
    }

    /// Blocks until the next event arrives.
    ///
    /// `None` means the queue itself failed, which ends the event loop rather
    /// than spinning on it.
    pub fn next(&self) -> Option<Event> {
        self.queue.get(FuriDuration::WAIT_FOREVER).ok()
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}
