//! Shared line-prefix grammar for heading / quote / list / task.
//! Parser, Enter, Backspace, and task toggle must all use this — nowhere else.

use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinePrefixKind {
    Heading { level: usize },
    Blockquote { depth: usize },
    Task { checked: bool, bullet: char },
    Unordered { bullet: char },
    Ordered { number: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinePrefix {
    pub kind: LinePrefixKind,
    /// Character length of the full prefix including leading indent spaces.
    pub prefix_len: usize,
    /// Leading spaces before the marker.
    pub indent: usize,
}

impl LinePrefix {
    pub fn range(&self) -> Range<usize> {
        0..self.prefix_len
    }

    /// Content after the prefix.
    pub fn content_owned(&self, line: &str) -> String {
        line.chars().skip(self.prefix_len).collect()
    }

    /// Marker text to continue on Enter (indent + marker), or None for headings.
    pub fn continue_prefix(&self) -> Option<String> {
        let indent: String = " ".repeat(self.indent);
        match &self.kind {
            LinePrefixKind::Heading { .. } => None,
            LinePrefixKind::Blockquote { .. } => Some(format!("{}> ", indent)),
            LinePrefixKind::Task { bullet, .. } => {
                Some(format!("{}{} [ ] ", indent, bullet))
            }
            LinePrefixKind::Unordered { bullet } => {
                Some(format!("{}{} ", indent, bullet))
            }
            LinePrefixKind::Ordered { number } => {
                Some(format!("{}{}. ", indent, number + 1))
            }
        }
    }

    /// Toggle checked state for a task line; returns the full replacement line.
    pub fn toggle_task_line(&self, line: &str) -> Option<String> {
        let LinePrefixKind::Task { checked, bullet } = &self.kind else {
            return None;
        };
        let content = self.content_owned(line);
        let indent: String = " ".repeat(self.indent);
        let new = if *checked {
            format!("{}{} [ ] {}", indent, bullet, content)
        } else {
            format!("{}{} [x] {}", indent, bullet, content)
        };
        Some(new)
    }
}

/// Detect heading / quote / list / task prefix. Does not handle fences or HR.
pub fn parse_prefix(line: &str) -> Option<LinePrefix> {
    let trimmed_end = line.trim_end_matches(['\r', '\n']);
    let chars: Vec<char> = trimmed_end.chars().collect();
    if chars.is_empty() {
        return None;
    }

    // Heading: #{1,6} followed by space
    if chars[0] == '#' {
        let mut level = 0;
        while level < chars.len() && chars[level] == '#' {
            level += 1;
        }
        if level >= 1 && level <= 6 && level < chars.len() && chars[level] == ' ' {
            return Some(LinePrefix {
                kind: LinePrefixKind::Heading { level },
                prefix_len: level + 1,
                indent: 0,
            });
        }
    }

    // Blockquote: one or more "> " / ">"
    if chars[0] == '>' {
        let mut depth = 0;
        let mut idx = 0;
        while idx < chars.len() && chars[idx] == '>' {
            depth += 1;
            idx += 1;
            if idx < chars.len() && chars[idx] == ' ' {
                idx += 1;
            }
        }
        if depth > 0 {
            return Some(LinePrefix {
                kind: LinePrefixKind::Blockquote { depth },
                prefix_len: idx,
                indent: 0,
            });
        }
    }

    let indent = chars.iter().take_while(|&&c| c == ' ').count();
    let after: String = chars[indent..].iter().collect();

    // Task: - [ ] / * [ ] / - [x] / * [X]
    if let Some(prefix) = parse_task(&after, indent) {
        return Some(prefix);
    }

    // Unordered: - / * / + followed by space
    if after.starts_with("- ") || after.starts_with("* ") || after.starts_with("+ ") {
        let bullet = after.chars().next().unwrap();
        return Some(LinePrefix {
            kind: LinePrefixKind::Unordered { bullet },
            prefix_len: indent + 2,
            indent,
        });
    }

    // Ordered: digits + ". "
    let digits_len = after.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_len > 0 {
        let rest: String = after.chars().skip(digits_len).collect();
        if rest.starts_with(". ") {
            let num_str: String = after.chars().take(digits_len).collect();
            let number: usize = num_str.parse().unwrap_or(1);
            return Some(LinePrefix {
                kind: LinePrefixKind::Ordered { number },
                prefix_len: indent + digits_len + 2,
                indent,
            });
        }
    }

    None
}

fn parse_task(after: &str, indent: usize) -> Option<LinePrefix> {
    let patterns = [
        ("- [ ] ", false, '-'),
        ("* [ ] ", false, '*'),
        ("- [x] ", true, '-'),
        ("- [X] ", true, '-'),
        ("* [x] ", true, '*'),
        ("* [X] ", true, '*'),
    ];
    for (pat, checked, bullet) in patterns {
        if after.starts_with(pat) {
            return Some(LinePrefix {
                kind: LinePrefixKind::Task { checked, bullet },
                prefix_len: indent + pat.chars().count(),
                indent,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_and_task() {
        let h = parse_prefix("### Title").unwrap();
        assert_eq!(h.kind, LinePrefixKind::Heading { level: 3 });
        assert_eq!(h.prefix_len, 4);

        let t = parse_prefix("  * [x] Done").unwrap();
        assert_eq!(
            t.kind,
            LinePrefixKind::Task {
                checked: true,
                bullet: '*'
            }
        );
        assert_eq!(t.indent, 2);
        assert_eq!(t.prefix_len, 8);
    }

    #[test]
    fn toggle_star_task() {
        let line = "* [ ] Hello";
        let p = parse_prefix(line).unwrap();
        assert_eq!(p.toggle_task_line(line).unwrap(), "* [x] Hello");
    }
}
