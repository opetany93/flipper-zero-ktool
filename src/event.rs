//! The single channel the application thread listens on.

use flipperzero::furi::message_queue::MessageQueue;
use flipperzero::furi::time::FuriDuration;

use crate::hal::input::InputEvent;

/// Everything that can wake the event loop.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    Input(InputEvent),
    Tick,
}

/// Comfortably more slots than events ever in flight, which is what lets
/// [`EventQueue::post`] block without ever actually waiting.
const CAPACITY: usize = 8;

/// A typed mailbox: written by the GUI and timer threads, drained by the
/// application thread.
pub struct EventQueue {
    queue: MessageQueue<Event>,
}

// SAFETY: a Furi message queue exists to be written from other threads, and
// every method below goes through `&self`. `MessageQueue` is `!Sync` only
// because it holds a raw pointer.
unsafe impl Sync for EventQueue {}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            queue: MessageQueue::new(CAPACITY),
        }
    }

    /// Posts an event, waiting for a slot. For the GUI thread, where input must
    /// not be lost.
    pub fn post(&self, event: Event) {
        let _ = self.queue.put(event, FuriDuration::WAIT_FOREVER);
    }

    /// Posts an event, dropping it if the queue is full. For the timer service
    /// thread, where blocking would stall every timer in the system.
    pub fn try_post(&self, event: Event) {
        let _ = self.queue.put(event, FuriDuration::ZERO);
    }

    /// Blocks until the next event. `None` means the queue failed, which ends
    /// the event loop.
    pub fn next(&self) -> Option<Event> {
        self.queue.get(FuriDuration::WAIT_FOREVER).ok()
    }
}
