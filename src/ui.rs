//! The one screen KTool currently draws.
//!
//! Pure rendering: no hardware, no lock, no clock. A second screen means a
//! second function here, not a change to the event loop.

use core::fmt;

use crate::hal::canvas::{Canvas, Font};
use crate::supply::Reading;
use crate::text::{TextBuffer, format_to_cstr};
use crate::units::Millivolts;

const MARGIN_X: i32 = 0;

/// Text baselines, top to bottom.
const TITLE_Y: i32 = 10;
const VS_Y: i32 = 26;
const B_PLUS_Y: i32 = 38;
const RAW_Y: i32 = 50;

/// Enough for the longest line, `B+   12.34 V`, plus the NUL terminator.
const LINE_CAPACITY: usize = 24;

/// A voltage, or `--` where there is no valid reading to show.
struct Volts(Option<Millivolts>);

impl fmt::Display for Volts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(mv) => write!(f, "{mv} V"),
            None => f.write_str("-- V"),
        }
    }
}

pub fn draw(canvas: &mut Canvas<'_>, reading: &Reading) {
    let mut line = TextBuffer::<LINE_CAPACITY>::new();

    canvas.clear();

    canvas.set_font(Font::Primary);
    canvas.draw_str(MARGIN_X, TITLE_Y, c"KTool");

    canvas.set_font(Font::Secondary);
    canvas.draw_str(
        MARGIN_X,
        VS_Y,
        format_to_cstr!(line, "VS   {}", Volts(reading.vs)),
    );
    canvas.draw_str(
        MARGIN_X,
        B_PLUS_Y,
        format_to_cstr!(line, "B+   {}", Volts(reading.b_plus)),
    );
    canvas.draw_str(
        MARGIN_X,
        RAW_Y,
        format_to_cstr!(line, "raw  {}", reading.adc_raw),
    );
}
