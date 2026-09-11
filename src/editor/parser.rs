use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Heading { level: usize },
    Blockquote { depth: usize },
    UnorderedList { indent: usize },
    OrderedList { number: usize, indent: usize },
    TaskList { checked: bool, indent: usize },
    CodeFence { language: Option<String> },
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
    pub char_range: Range<usize>, // 0-indexed character offsets within the line
    pub style: SpanStyle,
    pub group_range: Range<usize>, // Entire token boundary for proximity detection
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedLine {
    pub block_kind: BlockKind,
    pub prefix_marker_range: Option<Range<usize>>, // e.g. "### " or "- "
    pub spans: Vec<StyledSpan>,
    pub raw_text: String,
}

pub struct MarkdownParser;

impl MarkdownParser {
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

        // 1. Check for Code Fence ```
        if trimmed_end.starts_with("```") {
            let lang = trimmed_end[3..].trim();
            let lang_opt = if lang.is_empty() { None } else { Some(lang.to_string()) };
            return ParsedLine {
                block_kind: BlockKind::CodeFence { language: lang_opt },
                prefix_marker_range: Some(0..char_count),
                spans: vec![StyledSpan {
                    char_range: 0..char_count,
                    style: SpanStyle::Marker(MarkerType::CodeFence),
                    group_range: 0..char_count,
                }],
                raw_text: trimmed_end.to_string(),
            };
        }

        // 2. Check for Horizontal Rule (--- or *** or ___)
        let trimmed_space = trimmed_end.trim();
        if (trimmed_space == "---" || trimmed_space == "***" || trimmed_space == "___") 
            && trimmed_space.len() >= 3 
        {
            return ParsedLine {
                block_kind: BlockKind::HorizontalRule,
                prefix_marker_range: Some(0..char_count),
                spans: vec![StyledSpan {
                    char_range: 0..char_count,
                    style: SpanStyle::Marker(MarkerType::CodeFence),
                    group_range: 0..char_count,
                }],
                raw_text: trimmed_end.to_string(),
            };
        }

        // 3. Check for Headings (# .. ######)
        if trimmed_end.starts_with('#') {
            let mut level = 0;
            while level < chars.len() && chars[level] == '#' {
                level += 1;
            }
            if level <= 6 && level < chars.len() && chars[level] == ' ' {
                let prefix_len = level + 1; // including trailing space
                let mut spans = vec![StyledSpan {
                    char_range: 0..prefix_len,
                    style: SpanStyle::Marker(MarkerType::HeadingPrefix),
                    group_range: 0..char_count,
                }];
                // Parse inline spans in heading content
                let content_str: String = chars[prefix_len..].iter().collect();
                let mut content_spans = Self::parse_inlines(&content_str, prefix_len);
                spans.append(&mut content_spans);

                return ParsedLine {
                    block_kind: BlockKind::Heading { level },
                    prefix_marker_range: Some(0..prefix_len),
                    spans,
                    raw_text: trimmed_end.to_string(),
                };
            }
        }

        // 4. Check for Blockquote (> )
        if trimmed_end.starts_with('>') {
            let mut depth = 0;
            let mut idx = 0;
            while idx < chars.len() && chars[idx] == '>' {
                depth += 1;
                idx += 1;
                if idx < chars.len() && chars[idx] == ' ' {
                    idx += 1;
                }
            }
            let mut spans = vec![StyledSpan {
                char_range: 0..idx,
                style: SpanStyle::Marker(MarkerType::BlockquotePrefix),
                group_range: 0..idx,
            }];
            let content_str: String = chars[idx..].iter().collect();
            let mut content_spans = Self::parse_inlines(&content_str, idx);
            spans.append(&mut content_spans);

            return ParsedLine {
                block_kind: BlockKind::Blockquote { depth },
                prefix_marker_range: Some(0..idx),
                spans,
                raw_text: trimmed_end.to_string(),
            };
        }

        // 5. Check for Task List / Checklist (- [ ] or - [x])
        let indent = chars.iter().take_while(|&&c| c == ' ').count();
        let after_indent = &trimmed_end[indent..];
        if after_indent.starts_with("- [ ] ") || after_indent.starts_with("* [ ] ") {
            let prefix_len = indent + 6;
            let mut spans = vec![StyledSpan {
                char_range: 0..prefix_len,
                style: SpanStyle::Marker(MarkerType::TaskListMarker { checked: false }),
                group_range: 0..prefix_len,
            }];
            let content_str: String = chars[prefix_len..].iter().collect();
            let mut content_spans = Self::parse_inlines(&content_str, prefix_len);
            spans.append(&mut content_spans);

            return ParsedLine {
                block_kind: BlockKind::TaskList { checked: false, indent },
                prefix_marker_range: Some(0..prefix_len),
                spans,
                raw_text: trimmed_end.to_string(),
            };
        }
        if after_indent.starts_with("- [x] ") 
            || after_indent.starts_with("- [X] ") 
            || after_indent.starts_with("* [x] ") 
            || after_indent.starts_with("* [X] ") 
        {
            let prefix_len = indent + 6;
            let mut spans = vec![StyledSpan {
                char_range: 0..prefix_len,
                style: SpanStyle::Marker(MarkerType::TaskListMarker { checked: true }),
                group_range: 0..prefix_len,
            }];
            let content_str: String = chars[prefix_len..].iter().collect();
            let mut content_spans = Self::parse_inlines(&content_str, prefix_len);
            spans.append(&mut content_spans);

            return ParsedLine {
                block_kind: BlockKind::TaskList { checked: true, indent },
                prefix_marker_range: Some(0..prefix_len),
                spans,
                raw_text: trimmed_end.to_string(),
            };
        }

        // 6. Check for Unordered List (- or * or +)
        if after_indent.starts_with("- ") || after_indent.starts_with("* ") || after_indent.starts_with("+ ") {
            let prefix_len = indent + 2;
            let mut spans = vec![StyledSpan {
                char_range: 0..prefix_len,
                style: SpanStyle::Marker(MarkerType::ListPrefix),
                group_range: 0..prefix_len,
            }];
            let content_str: String = chars[prefix_len..].iter().collect();
            let mut content_spans = Self::parse_inlines(&content_str, prefix_len);
            spans.append(&mut content_spans);

            return ParsedLine {
                block_kind: BlockKind::UnorderedList { indent },
                prefix_marker_range: Some(0..prefix_len),
                spans,
                raw_text: trimmed_end.to_string(),
            };
        }

        // 7. Check for Ordered List (1. or 2. etc.)
        let digits_len = after_indent.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits_len > 0 && after_indent[digits_len..].starts_with(". ") {
            let num: usize = after_indent[..digits_len].parse().unwrap_or(1);
            let prefix_len = indent + digits_len + 2;
            let mut spans = vec![StyledSpan {
                char_range: 0..prefix_len,
                style: SpanStyle::Marker(MarkerType::OrderedListPrefix { number: num }),
                group_range: 0..prefix_len,
            }];
            let content_str: String = chars[prefix_len..].iter().collect();
            let mut content_spans = Self::parse_inlines(&content_str, prefix_len);
            spans.append(&mut content_spans);

            return ParsedLine {
                block_kind: BlockKind::OrderedList { number: num, indent },
                prefix_marker_range: Some(0..prefix_len),
                spans,
                raw_text: trimmed_end.to_string(),
            };
        }

        // 8. Normal Paragraph
        let spans = Self::parse_inlines(trimmed_end, 0);
        ParsedLine {
            block_kind: BlockKind::Paragraph,
            prefix_marker_range: None,
            spans,
            raw_text: trimmed_end.to_string(),
        }
    }

    /// Parse inline formatting: `**bold**`, `*italic*`, `~~strike~~`, `` `code` ``, `[text](url)`
    pub fn parse_inlines(text: &str, base_offset: usize) -> Vec<StyledSpan> {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut spans = Vec::new();
        let mut i = 0;
        let mut plain_start = 0;

        while i < len {
            // 1. Inline Code: `...`
            if chars[i] == '`' {
                // Flush plain text before
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
                    // Marker opening `
                    spans.push(StyledSpan {
                        char_range: (base_offset + code_start)..(base_offset + code_start + 1),
                        style: SpanStyle::Marker(MarkerType::InlineCodeMarker),
                        group_range: token_group.clone(),
                    });
                    // Content
                    if i > code_start + 1 {
                        spans.push(StyledSpan {
                            char_range: (base_offset + code_start + 1)..(base_offset + i),
                            style: SpanStyle::InlineCode,
                            group_range: token_group.clone(),
                        });
                    }
                    // Marker closing `
                    spans.push(StyledSpan {
                        char_range: (base_offset + i)..(base_offset + i + 1),
                        style: SpanStyle::Marker(MarkerType::InlineCodeMarker),
                        group_range: token_group,
                    });
                    i += 1;
                    plain_start = i;
                    continue;
                } else {
                    // Unmatched backtick: treat as plain
                    plain_start = code_start;
                    break;
                }
            }

            // 2. Bold/Italic: ***, **, or *
            if chars[i] == '*' || chars[i] == '_' {
                let marker_char = chars[i];
                let marker_count = chars[i..].iter().take_while(|&&c| c == marker_char).count();

                if marker_count >= 2 {
                    // Check for Bold (** or __)
                    if let Some(close_pos) = Self::find_matching_delimiter(&chars, i + 2, marker_char, 2) {
                        if i > plain_start {
                            let range = (base_offset + plain_start)..(base_offset + i);
                            spans.push(StyledSpan {
                                char_range: range.clone(),
                                style: SpanStyle::Plain,
                                group_range: range,
                            });
                        }
                        let token_group = (base_offset + i)..(base_offset + close_pos + 2);
                        // Opening marker **
                        spans.push(StyledSpan {
                            char_range: (base_offset + i)..(base_offset + i + 2),
                            style: SpanStyle::Marker(MarkerType::BoldMarker),
                            group_range: token_group.clone(),
                        });
                        // Content
                        if close_pos > i + 2 {
                            spans.push(StyledSpan {
                                char_range: (base_offset + i + 2)..(base_offset + close_pos),
                                style: SpanStyle::Bold,
                                group_range: token_group.clone(),
                            });
                        }
                        // Closing marker **
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

                // Check for Italic (* or _)
                if marker_count >= 1 {
                    if let Some(close_pos) = Self::find_matching_delimiter(&chars, i + 1, marker_char, 1) {
                        if i > plain_start {
                            let range = (base_offset + plain_start)..(base_offset + i);
                            spans.push(StyledSpan {
                                char_range: range.clone(),
                                style: SpanStyle::Plain,
                                group_range: range,
                            });
                        }
                        let token_group = (base_offset + i)..(base_offset + close_pos + 1);
                        // Opening marker *
                        spans.push(StyledSpan {
                            char_range: (base_offset + i)..(base_offset + i + 1),
                            style: SpanStyle::Marker(MarkerType::ItalicMarker),
                            group_range: token_group.clone(),
                        });
                        // Content
                        if close_pos > i + 1 {
                            spans.push(StyledSpan {
                                char_range: (base_offset + i + 1)..(base_offset + close_pos),
                                style: SpanStyle::Italic,
                                group_range: token_group.clone(),
                            });
                        }
                        // Closing marker *
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
                    // Opening [
                    spans.push(StyledSpan {
                        char_range: (base_offset + i)..(base_offset + i + 1),
                        style: SpanStyle::Marker(MarkerType::LinkBracket),
                        group_range: token_group.clone(),
                    });
                    // Text
                    let url_str: String = chars[url_start..url_end].iter().collect();
                    spans.push(StyledSpan {
                        char_range: (base_offset + i + 1)..(base_offset + text_end),
                        style: SpanStyle::LinkText { url: url_str },
                        group_range: token_group.clone(),
                    });
                    // Closing ]( and url and )
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

        // Trailing plain text
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

    fn find_matching_delimiter(chars: &[char], start: usize, delim: char, count: usize) -> Option<usize> {
        let mut i = start;
        let len = chars.len();
        while i + count <= len {
            let matches = (0..count).all(|offset| chars[i + offset] == delim);
            if matches {
                // Ensure not preceded by backslash escape
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let line = MarkdownParser::parse_line("### My Title");
        assert_eq!(line.block_kind, BlockKind::Heading { level: 3 });
        assert_eq!(line.prefix_marker_range, Some(0..4));
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].style, SpanStyle::Marker(MarkerType::HeadingPrefix));
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
        assert_eq!(line.block_kind, BlockKind::TaskList { checked: true, indent: 0 });
        assert_eq!(line.prefix_marker_range, Some(0..6));
    }

    #[test]
    fn test_parse_ordered_list() {
        let line = MarkdownParser::parse_line("12. Item twelve");
        assert_eq!(line.block_kind, BlockKind::OrderedList { number: 12, indent: 0 });
        assert_eq!(line.prefix_marker_range, Some(0..4));
        assert_eq!(line.spans[0].style, SpanStyle::Marker(MarkerType::OrderedListPrefix { number: 12 }));
    }
}

