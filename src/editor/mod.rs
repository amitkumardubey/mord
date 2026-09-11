pub mod buffer;
pub mod decorator;
pub mod layout;
pub mod offset;
pub mod parser;
pub mod prefix;
pub mod selection;

pub use buffer::{DocumentBuffer, TextPoint};
pub use decorator::{ConcealMode, DecoratedLine, Decorator, VisualFontStyle, VisualFontWeight, VisualRun};
pub use layout::{LineHitResult, LinePaintTheme, ShapedLineInput};
pub use offset::{BufferCol, BufferOffset, VisualCol, byte_to_char_index, char_to_byte_index};
pub use parser::{BlockKind, MarkdownParser, MarkerType, ParsedLine, SpanStyle, StyledSpan};
pub use prefix::{parse_prefix, LinePrefix, LinePrefixKind};
pub use selection::{CursorManager, Selection};
