//! View ports: the application's window onto the GUI service.

use core::ffi::c_void;
use core::marker::PhantomData;

use flipperzero::gui::Gui;
use flipperzero_sys as sys;

use super::canvas::Canvas;
use super::input::InputEvent;

/// A fullscreen view port, registered with the GUI service for as long as it
/// lives.
///
/// Holding the callbacks' borrows is what guarantees they are detached before
/// anything they captured can go away.
pub struct ViewPort<'a> {
    // Declared first so that it is dropped last: `Drop::drop` below still needs
    // the GUI record open.
    gui: Gui,
    raw: *mut sys::ViewPort,
    _callbacks: PhantomData<&'a ()>,
}

impl<'a> ViewPort<'a> {
    /// Allocates a fullscreen view port and shows it.
    ///
    /// Both callbacks run on the GUI service thread, hence `Sync`, and neither
    /// may block: the service is waiting on them.
    pub fn fullscreen<D, I>(on_draw: &'a D, on_input: &'a I) -> Self
    where
        D: Fn(&mut Canvas<'_>) + Sync,
        I: Fn(InputEvent) + Sync,
    {
        unsafe extern "C" fn draw_trampoline<D: Fn(&mut Canvas<'_>)>(
            canvas: *mut sys::Canvas,
            context: *mut c_void,
        ) {
            // SAFETY: `context` is the `&D` registered below, kept borrowed for
            // as long as the callbacks are attached. `canvas` is valid only for
            // this call, which is exactly the lifetime the wrapper gets.
            let on_draw: &D = unsafe { &*context.cast() };
            let mut canvas = unsafe { Canvas::from_raw(canvas) };

            on_draw(&mut canvas);
        }

        unsafe extern "C" fn input_trampoline<I: Fn(InputEvent)>(
            event: *mut sys::InputEvent,
            context: *mut c_void,
        ) {
            // SAFETY: as above; `event` points at a live event for this call.
            let on_input: &I = unsafe { &*context.cast() };
            let event = unsafe { &*event };

            if let Some(event) = InputEvent::from_sys(event) {
                on_input(event);
            }
        }

        let gui = Gui::open();

        // SAFETY: both trampolines match the callback types the GUI service
        // expects, and the contexts they will be handed outlive this view port.
        let raw = unsafe {
            let raw = sys::view_port_alloc();
            sys::view_port_draw_callback_set(
                raw,
                Some(draw_trampoline::<D>),
                (on_draw as *const D).cast_mut().cast(),
            );
            sys::view_port_input_callback_set(
                raw,
                Some(input_trampoline::<I>),
                (on_input as *const I).cast_mut().cast(),
            );

            // Last: from here on the view port is live and callbacks can fire.
            sys::gui_add_view_port(gui.as_ptr(), raw, sys::GuiLayerFullscreen);

            raw
        };

        Self {
            gui,
            raw,
            _callbacks: PhantomData,
        }
    }

    /// Asks the GUI service to redraw the view. Returns before it happens.
    pub fn request_redraw(&self) {
        // SAFETY: `self.raw` is valid for the lifetime of `self`.
        unsafe { sys::view_port_update(self.raw) };
    }
}

impl Drop for ViewPort<'_> {
    fn drop(&mut self) {
        // Disable first - that is what stops further callbacks.
        //
        // SAFETY: `self.raw` came from `view_port_alloc` and is freed exactly
        // once, while `self.gui` is still open.
        unsafe {
            sys::view_port_enabled_set(self.raw, false);
            sys::gui_remove_view_port(self.gui.as_ptr(), self.raw);
            sys::view_port_free(self.raw);
        }
    }
}
