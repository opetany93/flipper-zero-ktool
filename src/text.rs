//! Formatting text for the C drawing API without touching the heap.

use core::ffi::CStr;
use core::fmt::{self, Write};

/// A fixed-capacity buffer that formats Rust values into a C string.
///
/// `canvas_draw_str` wants a NUL-terminated pointer, and a draw callback has no
/// business allocating one. `N` is the total capacity, terminator included.
pub struct TextBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> TextBuffer<N> {
    /// Creates an empty buffer.
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    /// Formats `args`, replacing any previous content, and returns the result
    /// ready to hand to C.
    ///
    /// Text that does not fit is truncated: a clipped label is a better outcome
    /// on a 128x64 screen than a dropped frame.
    pub fn format(&mut self, args: fmt::Arguments<'_>) -> &CStr {
        self.len = 0;
        // The only way this fails is truncation, which is already the documented
        // behaviour.
        let _ = self.write_fmt(args);

        if let Some(terminator) = self.bytes.get_mut(self.len) {
            *terminator = 0;
        }

        CStr::from_bytes_until_nul(&self.bytes).unwrap_or(c"")
    }
}

impl<const N: usize> Default for TextBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Write for TextBuffer<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // One byte is always held back for the terminator.
        let room = N.saturating_sub(1 + self.len);
        let fits = room.min(s.len());

        // Copied by hand: `copy_from_slice` compiles to `memcpy`, which a FAP has
        // no libc to supply, so `compiler_builtins` links its own in. Measured at
        // 2.4 KB to move a dozen bytes.
        for (slot, byte) in self
            .bytes
            .iter_mut()
            .skip(self.len)
            .take(fits)
            .zip(s.as_bytes())
        {
            *slot = *byte;
        }
        self.len += fits;

        // Reporting the overflow is what stops `write_fmt` from formatting the
        // rest of the arguments into a buffer that cannot hold them.
        if fits == s.len() {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}
