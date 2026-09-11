use crate::editor::prefix::{parse_prefix, LinePrefixKind};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Heading { level: usize },
    Blockquote { depth: usize },
    UnorderedList { indent: usize },
    OrderedList { number: usize, indent: usize },
    TaskList { checked: bool, indent: usize },
    /// Opening or closing fence line (` ``` `).
    CodeFence { language: Option<String> },
    /// Line inside an open fence — no inline Markdown.
    CodeBody,
    HorizontalRule,
    Paragraph,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerType {
    HeadingPrefix,
    BlockquotePrefix,
    ListPrefix,
    OrderedListPrefix { number: usize },
    TaskListMarker { checked: bool },
    CodeFence,
    HorizontalRule,
    BoldMarker,
    ItalicMarker,
    StrikethroughMarker,
    InlineCodeMarker,
    LinkBracket,
    LinkUrl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanStyle {
    Plain,
    Marker(MarkerType),
    Bold,
    Italic,
    BoldItalic,
    InlineCode,
    Strikethrough,
    LinkText { url: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub char_range: Range<usize>,
    pub style: SpanStyle,
    pub group_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine {
    pub block_kind: BlockKind,
    pub prefix_marker_range: Option<Range<usize>>,
    pub spans: Vec<StyledSpan>,
    pub raw_text: String,
}

pub struct MarkdownParser;

impl MarkdownParser {
    /// Parse every line with fence-open state so body lines skip inline conceal.
    pub fn parse_document(lines: &[String]) -> Vec<ParsedLine> {
        let mut out = Vec::with_capacity(lines.len());
        let mut in_fence = false;
        for line in lines {
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if in_fence {
                if is_fence_line(trimmed) {
                    out.push(parse_fence_line(trimmed));
                    in_fence = false;
                } else {
                    out.push(parse_code_body(trimmed));
                }
            } else if is_fence_line(trimmed) {
                out.push(parse_fence_line(trimmed));
                in_fence = true;
            } else {
                out.push(Self::parse_line(trimmed));
            }
        }
        out
    }

    pub fn parse_line(line: &str) -> ParsedLine {
        let trimmed_end = line.trim_end_matches(['\r', '\n']);
        let chars: Vec<char> = trimmed_end.chars().collect();
        let char_count = chars.len();

        if char_count == 0 {
            return ParsedLine {
                block_kind: BlockKind::Paragraph,
                prefix_marker_range: None,
                spans: Vec::new(),
                raw_text: String::new(),
            };
        }

        // Fence line (document walk handles open state; single-line parse still recognizes it)
        if is_fence_line(trimmed_end) {
            return parse_fence_line(trimmed_end);
        }

        // Horizontal rule
        let trimmed_space = trimmed_end.trim();
        if is_horizontal_rule(trimmed_space) {
            return ParsedLine {
                block_kind: BlockKind::HorizontalRule,
                prefix_marker_range: Some(0..char_count),
                spans: vec![StyledSpan {
                    char_range: 0..char_count,
                    style: SpanStyle::Marker(MarkerType::HorizontalRule),
                    group_range: 0..char_count,
                }],
                raw_text: trimmed_end.to_string(),
            };
        }

        if let Some(prefix) = parse_prefix(trimmed_end) {
            return parse_prefixed_line(trimmed_end, &chars, &prefix);
        }

        let spans = Self::parse_inlines(trimmed_end, 0);
        ParsedLine {
            block_kind: BlockKind::Paragraph,
            prefix_marker_range: None,
            spans,
            raw_text: trimmed_end.to_string(),
        }
    }

    /// Parse inline formatting: `**bold**`, `***bold italic***`, `*italic*`, etc.
    pub fn parse_inlines(text: &str, base_offset: usize) -> Vec<StyledSpan> {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut spans = Vec::new();
        let mut i = 0;
        let mut plain_start = 0;

        while i < len {
            // 1. Inline Code: `...`
            if chars[i] == '`' {
                if i > plain_start {
                    let range = (base_offset + plain_start)..(base_offset + i);
                    spans.push(StyledSpan {
                        char_range: range.clone(),
                        style: SpanStyle::Plain,
                        group_range: range,
                    });
                }
                let code_start = i;
                i += 1;
                let mut found_end = false;
                while i < len {
                    if chars[i] == '`' {
                        found_end = true;
                        break;
                    }
                    i += 1;
                }
                if found_end {
                    let token_group = (base_offset + code_start)..(base_offset + i + 1);
                    spans.push(StyledSpan {
                        char_range: (base_offset + code_start)..(base_offset + code_start + 1),
                        style: SpanStyle::Marker(MarkerType::InlineCodeMarker),
                        group_range: token_group.clone(),
                    });
                    if i > code_start + 1 {
                        spans.push(StyledSpan {
                            char_range: (base_offset + code_start + 1)..(base_offset + i),
                            style: SpanStyle::InlineCode,
                            group_range: token_group.clone(),
                        });
                    }
                    spans.push(StyledSpan {
                        char_range: (base_offset + i)..(base_offset + i + 1),
                        style: SpanStyle::Marker(MarkerType::InlineCodeMarker),
                        group_range: token_group,
                    });
                    i += 1;
                    plain_start = i;
                    continue;
                } else {
                    // Unmatched backtick: keep as plain and continue the rest of the line
                    plain_start = code_start;
                    i = code_start + 1;
                    continue;
                }
            }

            // 2. Bold/Italic/BoldItalic: ***, **, or *
            if chars[i] == '*' || chars[i] == '_' {
                let marker_char = chars[i];
                let marker_count = chars[i..]
                    .iter()
                    .take_while(|&&c| c == marker_char)
                    .count();

                // Bold-italic *** / ___
                if marker_count >= 3 {
                    if let Some(close_pos) =
                        Self::find_matching_delimiter(&chars, i + 3, marker_char, 3)
                    {
                        if i > plain_start {
                            let range = (base_offset + plain_start)..(base_offset + i);
                            spans.push(StyledSpan {
                                char_range: range.clone(),
                                style: SpanStyle::Plain,
                                group_range: range,
                            });
                        }
                        let token_group = (base_offset + i)..(base_offset + close_pos + 3);
                        spans.push(StyledSpan {
                            char_range: (base_offset + i)..(base_offset + i + 3),
                            style: SpanStyle::Marker(MarkerType::BoldMarker),
                            group_range: token_group.clone(),
                        });
                        if close_pos > i + 3 {
                            spans.push(StyledSpan {
                                char_range: (base_offset + i + 3)..(base_offset + close_pos),
                                style: SpanStyle::BoldItalic,
                                group_range: token_group.clone(),
                            });
                        }
                        spans.push(StyledSpan {
                            char_range: (base_offset + close_pos)..(base_offset + close_pos + 3),
                            style: SpanStyle::Marker(MarkerType::BoldMarker),
                            group_range: token_group,
                        });
                        i = close_pos + 3;
                        plain_start = i;
                        continue;
                    }
                }

                if marker_count >= 2 {
                    if let Some(close_pos) =
                        Self::find_matching_delimiter(&chars, i + 2, marker_char, 2)
                    {
                        if i > plain_start {
                            let range = (base_offset + plain_start)..(base_offset + i);
                            spans.push(StyledSpan {
                                char_range: range.clone(),
                                style: SpanStyle::Plain,
                                group_range: range,
                            });
                        }
                        let token_group = (base_offset + i)..(base_offset + close_pos + 2);
                        spans.push(StyledSpan {
                            char_range: (base_offset + i)..(base_offset + i + 2),
                            style: SpanStyle::Marker(MarkerType::BoldMarker),
                            group_range: token_group.clone(),
                        });
                        if close_pos > i + 2 {
                            spans.push(StyledSpan {
                                char_range: (base_offset + i + 2)..(base_offset + close_pos),
                                style: SpanStyle::Bold,
                                group_range: token_group.clone(),
                            });
                        }
                        spans.push(StyledSpan {
                            char_range: (base_offset + close_pos)..(base_offset + close_pos + 2),
                            style: SpanStyle::Marker(MarkerType::BoldMarker),
                            group_range: token_group,
                        });
                        i = close_pos + 2;
                        plain_start = i;
                        continue;
                    }
                }

                if marker_count >= 1 {
                    if let Some(close_pos) =
                        Self::find_matching_delimiter(&chars, i + 1, marker_char, 1)
                    {
                        if i > plain_start {
                            let range = (base_offset + plain_start)..(base_offset + i);
                            spans.push(StyledSpan {
                                char_range: range.clone(),
                                style: SpanStyle::Plain,
                                group_range: range,
                            });
                        }
                        let token_group = (base_offset + i)..(base_offset + close_pos + 1);
                        spans.push(StyledSpan {
                            char_range: (base_offset + i)..(base_offset + i + 1),
                            style: SpanStyle::Marker(MarkerType::ItalicMarker),
                            group_range: token_group.clone(),
                        });
                        if close_pos > i + 1 {
                            spans.push(StyledSpan {
                                char_range: (base_offset + i + 1)..(base_offset + close_pos),
                                style: SpanStyle::Italic,
                                group_range: token_group.clone(),
                            });
                        }
                        spans.push(StyledSpan {
                            char_range: (base_offset + close_pos)..(base_offset + close_pos + 1),
                            style: SpanStyle::Marker(MarkerType::ItalicMarker),
                            group_range: token_group,
                        });
                        i = close_pos + 1;
                        plain_start = i;
                        continue;
                    }
                }
            }

            // 3. Strikethrough: ~~...~~
            if chars[i] == '~' && i + 1 < len && chars[i + 1] == '~' {
                if let Some(close_pos) = Self::find_matching_delimiter(&chars, i + 2, '~', 2) {
                    if i > plain_start {
                        let range = (base_offset + plain_start)..(base_offset + i);
                        spans.push(StyledSpan {
                            char_range: range.clone(),
                            style: SpanStyle::Plain,
                            group_range: range,
                        });
                    }
                    let token_group = (base_offset + i)..(base_offset + close_pos + 2);
                    spans.push(StyledSpan {
                        char_range: (base_offset + i)..(base_offset + i + 2),
                        style: SpanStyle::Marker(MarkerType::StrikethroughMarker),
                        group_range: token_group.clone(),
                    });
                    if close_pos > i + 2 {
                        spans.push(StyledSpan {
                            char_range: (base_offset + i + 2)..(base_offset + close_pos),
                            style: SpanStyle::Strikethrough,
                            group_range: token_group.clone(),
                        });
                    }
                    spans.push(StyledSpan {
                        char_range: (base_offset + close_pos)..(base_offset + close_pos + 2),
                        style: SpanStyle::Marker(MarkerType::StrikethroughMarker),
                        group_range: token_group,
                    });
                    i = close_pos + 2;
                    plain_start = i;
                    continue;
                }
            }

            // 4. Link: [text](url)
            if chars[i] == '[' {
                if let Some((text_end, url_start, url_end)) = Self::find_link(&chars, i) {
                    if i > plain_start {
                        let range = (base_offset + plain_start)..(base_offset + i);
                        spans.push(StyledSpan {
                            char_range: range.clone(),
                            style: SpanStyle::Plain,
                            group_range: range,
                        });
                    }
                    let token_group = (base_offset + i)..(base_offset + url_end + 1);
                    spans.push(StyledSpan {
                        char_range: (base_offset + i)..(base_offset + i + 1),
                        style: SpanStyle::Marker(MarkerType::LinkBracket),
                        group_range: token_group.clone(),
                    });
                    let url_str: String = chars[url_start..url_end].iter().collect();
                    spans.push(StyledSpan {
                        char_range: (base_offset + i + 1)..(base_offset + text_end),
                        style: SpanStyle::LinkText { url: url_str },
                        group_range: token_group.clone(),
                    });
                    spans.push(StyledSpan {
                        char_range: (base_offset + text_end)..(base_offset + url_end + 1),
                        style: SpanStyle::Marker(MarkerType::LinkUrl),
                        group_range: token_group,
                    });
                    i = url_end + 1;
                    plain_start = i;
                    continue;
                }
            }

            i += 1;
        }

        if plain_start < len {
            let range = (base_offset + plain_start)..(base_offset + len);
            spans.push(StyledSpan {
                char_range: range.clone(),
                style: SpanStyle::Plain,
                group_range: range,
            });
        }

        spans
    }

    fn find_matching_delimiter(
        chars: &[char],
        start: usize,
        delim: char,
        count: usize,
    ) -> Option<usize> {
        let mut i = start;
        let len = chars.len();
        while i + count <= len {
            let matches = (0..count).all(|offset| chars[i + offset] == delim);
            if matches {
                if i > 0 && chars[i - 1] == '\\' {
                    i += count;
                    continue;
                }
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_link(chars: &[char], open_bracket: usize) -> Option<(usize, usize, usize)> {
        let len = chars.len();
        let mut i = open_bracket + 1;
        let mut text_end = None;
        while i < len {
            if chars[i] == ']' {
                text_end = Some(i);
                break;
            }
            i += 1;
        }
        let text_end_idx = text_end?;
        if text_end_idx + 1 < len && chars[text_end_idx + 1] == '(' {
            let url_start = text_end_idx + 2;
            let mut j = url_start;
            while j < len {
                if chars[j] == ')' {
                    return Some((text_end_idx, url_start, j));
                }
                j += 1;
            }
        }
        None
    }
}

fn is_fence_line(line: &str) -> bool {
    line.starts_with("```")
}

fn is_horizontal_rule(trimmed: &str) -> bool {
    (trimmed == "---" || trimmed == "***" || trimmed == "___") && trimmed.len() >= 3
}

fn parse_fence_line(trimmed_end: &str) -> ParsedLine {
    let char_count = trimmed_end.chars().count();
    let lang = trimmed_end.chars().skip(3).collect::<String>();
    let lang = lang.trim();
    let lang_opt = if lang.is_empty() {
        None
    } else {
        Some(lang.to_string())
    };
    ParsedLine {
        block_kind: BlockKind::CodeFence {
            language: lang_opt,
        },
        prefix_marker_range: Some(0..char_count),
        spans: vec![StyledSpan {
            char_range: 0..char_count,
            style: SpanStyle::Marker(MarkerType::CodeFence),
            group_range: 0..char_count,
        }],
        raw_text: trimmed_end.to_string(),
    }
}

fn parse_code_body(trimmed_end: &str) -> ParsedLine {
    let char_count = trimmed_end.chars().count();
    let spans = if char_count == 0 {
        Vec::new()
    } else {
        vec![StyledSpan {
            char_range: 0..char_count,
            style: SpanStyle::InlineCode,
            group_range: 0..char_count,
        }]
    };
    ParsedLine {
        block_kind: BlockKind::CodeBody,
        prefix_marker_range: None,
        spans,
        raw_text: trimmed_end.to_string(),
    }
}

fn parse_prefixed_line(
    trimmed_end: &str,
    chars: &[char],
    prefix: &crate::editor::prefix::LinePrefix,
) -> ParsedLine {
    let prefix_len = prefix.prefix_len;
    let (block_kind, marker) = match &prefix.kind {
        LinePrefixKind::Heading { level } => (
            BlockKind::Heading { level: *level },
            MarkerType::HeadingPrefix,
        ),
        LinePrefixKind::Blockquote { depth } => (
            BlockKind::Blockquote { depth: *depth },
            MarkerType::BlockquotePrefix,
        ),
        LinePrefixKind::Task { checked, .. } => (
            BlockKind::TaskList {
                checked: *checked,
                indent: prefix.indent,
            },
            MarkerType::TaskListMarker {
                checked: *checked,
            },
        ),
        LinePrefixKind::Unordered { .. } => (
            BlockKind::UnorderedList {
                indent: prefix.indent,
            },
            MarkerType::ListPrefix,
        ),
        LinePrefixKind::Ordered { number } => (
            BlockKind::OrderedList {
                number: *number,
                indent: prefix.indent,
            },
            MarkerType::OrderedListPrefix { number: *number },
        ),
    };

    // Heading group_range is the prefix token only (TokenReveal)
    let group_range = 0..prefix_len;
    let mut spans = vec![StyledSpan {
        char_range: 0..prefix_len,
        style: SpanStyle::Marker(marker),
        group_range,
    }];
    if prefix_len < chars.len() {
        let content_str: String = chars[prefix_len..].iter().collect();
        spans.append(&mut MarkdownParser::parse_inlines(&content_str, prefix_len));
    }

    ParsedLine {
        block_kind,
        prefix_marker_range: Some(0..prefix_len),
        spans,
        raw_text: trimmed_end.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let line = MarkdownParser::parse_line("### My Title");
        assert_eq!(line.block_kind, BlockKind::Heading { level: 3 });
        assert_eq!(line.prefix_marker_range, Some(0..4));
        assert_eq!(line.spans.len(), 2);
        assert_eq!(
            line.spans[0].style,
            SpanStyle::Marker(MarkerType::HeadingPrefix)
        );
        assert_eq!(line.spans[0].group_range, 0..4);
        assert_eq!(line.spans[1].style, SpanStyle::Plain);
    }

    #[test]
    fn test_parse_bold_and_italic() {
        let spans = MarkdownParser::parse_inlines("Hello **bold world** and *italic*", 0);
        assert_eq!(spans.len(), 8);
        assert_eq!(spans[1].style, SpanStyle::Marker(MarkerType::BoldMarker));
        assert_eq!(spans[2].style, SpanStyle::Bold);
    }

    #[test]
    fn test_parse_task_list() {
        let line = MarkdownParser::parse_line("- [x] Complete task");
        assert_eq!(
            line.block_kind,
            BlockKind::TaskList {
                checked: true,
                indent: 0
            }
        );
        assert_eq!(line.prefix_marker_range, Some(0..6));
    }

    #[test]
    fn test_parse_ordered_list() {
        let line = MarkdownParser::parse_line("12. Item twelve");
        assert_eq!(
            line.block_kind,
            BlockKind::OrderedList {
                number: 12,
                indent: 0
            }
        );
        assert_eq!(line.prefix_marker_range, Some(0..4));
        assert_eq!(
            line.spans[0].style,
            SpanStyle::Marker(MarkerType::OrderedListPrefix { number: 12 })
        );
    }

    #[test]
    fn fence_body_skips_inlines() {
        let lines = vec![
            "```rust".into(),
            "let x = **not bold**;".into(),
            "```".into(),
        ];
        let doc = MarkdownParser::parse_document(&lines);
        assert!(matches!(doc[0].block_kind, BlockKind::CodeFence { .. }));
        assert_eq!(doc[1].block_kind, BlockKind::CodeBody);
        assert_eq!(doc[1].spans.len(), 1);
        assert_eq!(doc[1].spans[0].style, SpanStyle::InlineCode);
        assert!(matches!(doc[2].block_kind, BlockKind::CodeFence { .. }));
    }

    #[test]
    fn horizontal_rule_marker() {
        let line = MarkdownParser::parse_line("---");
        assert_eq!(line.block_kind, BlockKind::HorizontalRule);
        assert_eq!(
            line.spans[0].style,
            SpanStyle::Marker(MarkerType::HorizontalRule)
        );
    }

    #[test]
    fn unmatched_backtick_continues() {
        let spans = MarkdownParser::parse_inlines("`oops and *italic*", 0);
        assert!(spans.iter().any(|s| s.style == SpanStyle::Italic));
    }

    #[test]
    fn bold_italic_triple_star() {
        let spans = MarkdownParser::parse_inlines("***both***", 0);
        assert!(spans.iter().any(|s| s.style == SpanStyle::BoldItalic));
    }
}
