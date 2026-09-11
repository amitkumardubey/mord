//! Coordinate spaces for the live-inline editor.
//!
//! Buffer offsets and columns are Unicode scalar indices (ropey chars).
//! Visual columns index `DecoratedLine.display_text` the same way.
//! GPUI shaping uses UTF-8 bytes of display text; convert only at the layout boundary.

use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// Character offset into the document rope.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BufferOffset(pub usize);

/// Character column within a source line (excluding the trailing newline).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BufferCol(pub usize);

/// Character column within a line's visual `display_text`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct VisualCol(pub usize);

macro_rules! offset_impl {
    ($name:ident) => {
        impl $name {
            #[inline]
            pub const fn new(value: usize) -> Self {
                Self(value)
            }

            #[inline]
            pub const fn get(self) -> usize {
                self.0
            }

            #[inline]
            pub fn saturating_sub(self, other: Self) -> Self {
                Self(self.0.saturating_sub(other.0))
            }

            #[inline]
            pub fn min(self, other: Self) -> Self {
                Self(self.0.min(other.0))
            }

            #[inline]
            pub fn max(self, other: Self) -> Self {
                Self(self.0.max(other.0))
            }
        }

        impl From<usize> for $name {
            fn from(value: usize) -> Self {
                Self(value)
            }
        }

        impl From<$name> for usize {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl Add<usize> for $name {
            type Output = Self;
            fn add(self, rhs: usize) -> Self {
                Self(self.0 + rhs)
            }
        }

        impl AddAssign<usize> for $name {
            fn add_assign(&mut self, rhs: usize) {
                self.0 += rhs;
            }
        }

        impl Sub<usize> for $name {
            type Output = Self;
            fn sub(self, rhs: usize) -> Self {
                Self(self.0.saturating_sub(rhs))
            }
        }

        impl SubAssign<usize> for $name {
            fn sub_assign(&mut self, rhs: usize) {
                self.0 = self.0.saturating_sub(rhs);
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

offset_impl!(BufferOffset);
offset_impl!(BufferCol);
offset_impl!(VisualCol);

/// Map a Unicode scalar index into `text` to a UTF-8 byte index.
pub fn char_to_byte_index(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(text.len())
}

/// Map a UTF-8 byte index into `text` to a Unicode scalar index (clamped to a char boundary).
pub fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
    if byte_idx >= text.len() {
        return text.chars().count();
    }
    let mut clamped = byte_idx.min(text.len());
    while clamped > 0 && !text.is_char_boundary(clamped) {
        clamped -= 1;
    }
    text[..clamped].chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_byte_roundtrip_ascii() {
        let text = "Hello";
        assert_eq!(char_to_byte_index(text, 0), 0);
        assert_eq!(char_to_byte_index(text, 4), 4);
        assert_eq!(char_to_byte_index(text, 5), 5);
        assert_eq!(byte_to_char_index(text, 4), 4);
        assert_eq!(byte_to_char_index(text, 5), 5);
    }

    #[test]
    fn char_byte_multibyte() {
        let text = "café"; // é is 2 bytes
        assert_eq!(text.chars().count(), 4);
        assert_eq!(char_to_byte_index(text, 3), 3); // start of é
        assert_eq!(char_to_byte_index(text, 4), 5);
        assert_eq!(byte_to_char_index(text, 3), 3);
        assert_eq!(byte_to_char_index(text, 4), 3); // mid-char clamps down
        assert_eq!(byte_to_char_index(text, 5), 4);
    }
}
