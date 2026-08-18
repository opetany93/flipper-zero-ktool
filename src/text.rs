//! Formatting text for the C drawing API without touching the heap.

use core::ffi::CStr;
use core::fmt::{self, Write};

/// A fixed-capacity buffer that formats Rust values into a C string.
///
/// `N` is the total capacity, terminator included.
pub struct TextBuffer<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> TextBuffer<N> {
    pub const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    /// Formats `args`, replacing any previous content. Text that does not fit is
    /// truncated.
    pub fn format(&mut self, args: fmt::Arguments<'_>) -> &CStr {
        self.len = 0;
        let _ = self.write_fmt(args);

        if let Some(terminator) = self.bytes.get_mut(self.len) {
            *terminator = 0;
        }

        CStr::from_bytes_until_nul(&self.bytes).unwrap_or(c"")
    }
}

/// Formats into a [`TextBuffer`] and yields the result as a `&CStr`.
///
/// A macro rather than a method because only a macro can build a
/// `fmt::Arguments`, exactly as `write!` does for `write_fmt`.
macro_rules! format_to_cstr {
    ($buffer:expr, $($args:tt)*) => {
        $buffer.format(format_args!($($args)*))
    };
}

pub(crate) use format_to_cstr;

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

        // The `Err` is what stops `write_fmt` from formatting the rest.
        if fits == s.len() {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}
