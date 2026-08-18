//! The one screen KTool currently draws.
//!
//! Pure rendering: it is handed a finished [`SupplyReading`] and never reaches
//! for hardware, a lock or a clock. Adding a second screen means adding a
//! second function here, not touching the event loop.

use crate::hal::canvas::{Canvas, Font};
use crate::sensor::SupplyReading;
use crate::text::{TextBuffer, format_to_cstr};

/// Left margin shared by every line.
const MARGIN_X: i32 = 0;

/// Text baselines, top to bottom.
const TITLE_Y: i32 = 10;
const VS_Y: i32 = 26;
const B_PLUS_Y: i32 = 38;
const RAW_Y: i32 = 50;

/// Enough for the longest line, `B+   12.34 V`, plus the NUL terminator.
const LINE_CAPACITY: usize = 24;

/// Draws one frame.
pub fn draw(canvas: &mut Canvas<'_>, reading: &SupplyReading) {
    let mut line = TextBuffer::<LINE_CAPACITY>::new();

    canvas.clear();

    canvas.set_font(Font::Primary);
    canvas.draw_str(MARGIN_X, TITLE_Y, c"KTool");

    canvas.set_font(Font::Secondary);
    canvas.draw_str(
        MARGIN_X,
        VS_Y,
        format_to_cstr!(line, "VS   {} V", reading.vs),
    );
    canvas.draw_str(
        MARGIN_X,
        B_PLUS_Y,
        format_to_cstr!(line, "B+   {} V", reading.b_plus),
    );
    canvas.draw_str(
        MARGIN_X,
        RAW_Y,
        format_to_cstr!(line, "raw  {}", reading.adc_raw),
    );
}
