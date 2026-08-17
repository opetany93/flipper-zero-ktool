//! Drawing, limited to what KTool actually puts on screen.

use core::ffi::CStr;
use core::marker::PhantomData;

use flipperzero_sys as sys;

/// The system fonts KTool draws with.
#[derive(Clone, Copy, Debug)]
pub enum Font {
    /// Bold. Used for the title.
    Primary,
    /// Regular. Used for readouts.
    Secondary,
}

impl Font {
    const fn to_sys(self) -> sys::Font {
        match self {
            Self::Primary => sys::FontPrimary,
            Self::Secondary => sys::FontSecondary,
        }
    }
}

/// A borrowed drawing surface, valid for the duration of one draw callback.
///
/// The lifetime is the point: it is what stops a canvas from being stashed in a
/// struct somewhere and used after the GUI service has moved on to the next
/// frame.
pub struct Canvas<'a> {
    raw: *mut sys::Canvas,
    _frame: PhantomData<&'a mut sys::Canvas>,
}

impl Canvas<'_> {
    /// Wraps the canvas handed to a draw callback.
    ///
    /// # Safety
    ///
    /// `raw` must be a valid canvas, and the returned lifetime must not outlive
    /// the callback it was handed to.
    pub(crate) unsafe fn from_raw(raw: *mut sys::Canvas) -> Self {
        Self {
            raw,
            _frame: PhantomData,
        }
    }

    /// Blanks the frame.
    pub fn clear(&mut self) {
        // SAFETY: `self.raw` is valid for the lifetime of `self`, per the
        // contract of `from_raw`. The same holds for the calls below.
        unsafe { sys::canvas_clear(self.raw) };
    }

    /// Selects the font used by subsequent [`draw_str`](Self::draw_str) calls.
    pub fn set_font(&mut self, font: Font) {
        // SAFETY: see `clear`.
        unsafe { sys::canvas_set_font(self.raw, font.to_sys()) };
    }

    /// Draws `text` with its left edge at `x` and its baseline at `y`.
    pub fn draw_str(&mut self, x: i32, y: i32, text: &CStr) {
        // SAFETY: see `clear`. `text` is NUL-terminated by construction and
        // outlives the call.
        unsafe { sys::canvas_draw_str(self.raw, x, y, text.as_ptr()) };
    }
}
