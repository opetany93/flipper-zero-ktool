//! View ports: the application's window onto the GUI service.

use core::ffi::c_void;
use core::marker::PhantomData;

use flipperzero::gui::Gui;
use flipperzero_sys as sys;

use super::canvas::Canvas;
use super::input::InputEvent;

/// What the GUI service asks of the application.
///
/// Both methods run on the GUI service thread while the application thread is
/// off doing something else. That is why they take `&self` and why the trait
/// requires `Sync`: anything they touch has to be shared state, not owned
/// state.
pub trait View: Sync {
    /// Renders one frame.
    fn draw(&self, canvas: &mut Canvas<'_>);

    /// Handles a key event. Must not block - the GUI service is waiting on it.
    fn on_input(&self, event: InputEvent);
}

/// A fullscreen view port, registered with the GUI service for as long as it
/// lives.
///
/// It holds the borrow of its [`View`] on purpose: dropping the view port
/// detaches the callbacks from the GUI service, and the borrow is what
/// guarantees that happens before the view itself can go away.
pub struct ViewPort<'a> {
    // Declared first so that it is dropped last: `Drop::drop` below still needs
    // the GUI record open.
    gui: Gui,
    raw: *mut sys::ViewPort,
    _view: PhantomData<&'a ()>,
}

impl<'a> ViewPort<'a> {
    /// Allocates a fullscreen view port driven by `view`, and shows it.
    pub fn fullscreen<V: View>(view: &'a V) -> Self {
        unsafe extern "C" fn on_draw<V: View>(canvas: *mut sys::Canvas, context: *mut c_void) {
            // SAFETY: `context` is the `&V` registered below, which the
            // `ViewPort` keeps borrowed for as long as the callbacks are
            // attached. `canvas` is valid for the duration of this call, which
            // is exactly the lifetime the wrapper is given.
            let view: &V = unsafe { &*context.cast() };
            let mut canvas = unsafe { Canvas::from_raw(canvas) };

            view.draw(&mut canvas);
        }

        unsafe extern "C" fn on_input<V: View>(event: *mut sys::InputEvent, context: *mut c_void) {
            // SAFETY: as in `on_draw`; `event` points at a live event for the
            // duration of the call.
            let view: &V = unsafe { &*context.cast() };
            let event = unsafe { &*event };

            if let Some(event) = InputEvent::from_sys(event) {
                view.on_input(event);
            }
        }

        let context = (view as *const V).cast_mut().cast::<c_void>();
        let gui = Gui::open();

        // SAFETY: both trampolines match the callback types the GUI service
        // expects, and the context they will be handed outlives this view port.
        let raw = unsafe {
            let raw = sys::view_port_alloc();
            sys::view_port_draw_callback_set(raw, Some(on_draw::<V>), context);
            sys::view_port_input_callback_set(raw, Some(on_input::<V>), context);

            // Last: from here on the view port is live and callbacks can fire.
            sys::gui_add_view_port(gui.as_ptr(), raw, sys::GuiLayerFullscreen);

            raw
        };

        Self {
            gui,
            raw,
            _view: PhantomData,
        }
    }

    /// Asks the GUI service to redraw the view.
    pub fn request_redraw(&self) {
        // SAFETY: `self.raw` is valid for the lifetime of `self`.
        unsafe { sys::view_port_update(self.raw) };
    }
}

impl Drop for ViewPort<'_> {
    fn drop(&mut self) {
        // Disable first - that is what stops further callbacks. Only then is it
        // safe to detach and free.
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
