//! Periodic timers.

use core::ffi::c_void;
use core::marker::PhantomData;

use flipperzero::furi::time::FuriDuration;
use flipperzero_sys as sys;

/// A periodic Furi timer bound to a callback.
///
/// The callback runs on the timer service thread, which is shared by every
/// timer in the system - hence the `Sync` bound, and hence the rule that it
/// must not block.
///
/// The borrow of the callback is part of the type, so the timer provably cannot
/// outlive the thing it calls.
pub struct PeriodicTimer<'a, F> {
    timer: *mut sys::FuriTimer,
    _callback: PhantomData<&'a F>,
}

impl<'a, F: Fn() + Sync> PeriodicTimer<'a, F> {
    /// Allocates a stopped timer that will call `on_tick`.
    pub fn new(on_tick: &'a F) -> Self {
        /// Bridges the C callback signature back to `F`.
        unsafe extern "C" fn trampoline<F: Fn()>(context: *mut c_void) {
            // SAFETY: `context` is the `&F` handed to `furi_timer_alloc` below,
            // which this `PeriodicTimer` keeps borrowed for as long as the
            // timer exists.
            let on_tick: &F = unsafe { &*context.cast() };

            on_tick();
        }

        // SAFETY: the trampoline has the signature `FuriTimerCallback` requires,
        // and the context it will be given outlives the timer by construction.
        let timer = unsafe {
            sys::furi_timer_alloc(
                Some(trampoline::<F>),
                sys::FuriTimerTypePeriodic,
                (on_tick as *const F).cast_mut().cast::<c_void>(),
            )
        };

        Self {
            timer,
            _callback: PhantomData,
        }
    }

    /// Starts, or restarts, the timer with the given period.
    pub fn start(&mut self, period: FuriDuration) {
        // SAFETY: `self.timer` is valid for the lifetime of `self`.
        unsafe { sys::furi_timer_start(self.timer, period.as_ticks()) };
    }
}

impl<F> Drop for PeriodicTimer<'_, F> {
    fn drop(&mut self) {
        // Stop before freeing, so the trampoline cannot fire against a context
        // that is on its way out.
        //
        // SAFETY: the timer came from `furi_timer_alloc` and, since
        // `PeriodicTimer` is neither `Clone` nor `Copy`, is freed exactly once.
        unsafe {
            sys::furi_timer_stop(self.timer);
            sys::furi_timer_free(self.timer);
        }
    }
}
