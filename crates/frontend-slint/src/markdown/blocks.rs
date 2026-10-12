//! pulldown-cmark message parse into chat [`Block`]s.
//!
//! Fenced ```mermaid blocks become [`Block::Mermaid`]; every other fence
//! becomes [`Block::Code`]. Inline markup (emphasis, links, inline code)
//! folds into the enclosing block's plain text; unknown containers are
//! transparent so text is never lost.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// One rendered unit of a chat message body.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Para {
        text: String,
    },
    Code {
        lang: String,
        code: String,
    },
    Mermaid {
        source: String,
    },
    List {
        items: Vec<String>,
    },
    Quote {
        text: String,
    },
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

/// Open block-level frame while walking pulldown-cmark events.
enum Frame {
    Heading {
        level: u8,
        buf: String,
    },
    Para {
        buf: String,
    },
    Code {
        lang: String,
        buf: String,
    },
    Quote {
        buf: String,
    },
    List {
        items: Vec<String>,
        item: String,
    },
    Table {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
        row: Vec<String>,
        cell: String,
        in_cell: bool,
        in_head: bool,
    },
}

/// Split `text` into [`Block`]s (never empty for non-blank input).
pub fn parse_markdown(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let parser = Parser::new_ext(text, Options::ENABLE_TABLES);
    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                stack.push(Frame::Heading {
                    level: level_no(level),
                    buf: String::new(),
                });
            }
            Event::Start(Tag::Paragraph) => {
                if stack.is_empty() {
                    stack.push(Frame::Para { buf: String::new() });
                } else {
                    push_sep(&mut stack);
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match &kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split_whitespace().next().unwrap_or("").to_lowercase()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                stack.push(Frame::Code {
                    lang,
                    buf: String::new(),
                });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                stack.push(Frame::Quote { buf: String::new() });
            }
            Event::Start(Tag::List(_)) => {
                stack.push(Frame::List {
                    items: Vec::new(),
                    item: String::new(),
                });
            }
            Event::Start(Tag::Item) => push_sep(&mut stack),
            Event::Start(Tag::Table(_)) => stack.push(Frame::Table {
                head: Vec::new(),
                rows: Vec::new(),
                row: Vec::new(),
                cell: String::new(),
                in_cell: false,
                in_head: false,
            }),
            Event::Start(Tag::TableHead) => {
                if let Some(Frame::Table { in_head, .. }) = stack.last_mut() {
                    *in_head = true;
                }
            }
            Event::Start(Tag::TableRow) => {
                if let Some(Frame::Table { row, .. }) = stack.last_mut() {
                    row.clear();
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some(Frame::Table { cell, in_cell, .. }) = stack.last_mut() {
                    cell.clear();
                    *in_cell = true;
                }
            }
            Event::Start(_) => {}
            Event::End(TagEnd::Paragraph) => {
                if matches!(stack.last(), Some(Frame::Para { .. })) {
                    pop_frame(&mut stack, &mut blocks);
                }
            }
            Event::End(TagEnd::Heading(_))
            | Event::End(TagEnd::BlockQuote(_))
            | Event::End(TagEnd::CodeBlock)
            | Event::End(TagEnd::List(_))
            | Event::End(TagEnd::Table) => {
                pop_frame(&mut stack, &mut blocks);
            }
            Event::End(TagEnd::Item) => {
                if let Some(Frame::List { items, item }) = stack.last_mut() {
                    flush_item(items, std::mem::take(item));
                }
            }
            Event::End(TagEnd::TableHead) => {
                if let Some(Frame::Table { in_head, .. }) = stack.last_mut() {
                    *in_head = false;
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let Some(Frame::Table {
                    head,
                    rows,
                    row,
                    in_head,
                    ..
                }) = stack.last_mut()
                {
                    let done: Vec<String> = std::mem::take(row);
                    if *in_head {
                        *head = done;
                    } else {
                        rows.push(done);
                    }
                }
            }
            Event::End(TagEnd::TableCell) => {
                if let Some(Frame::Table {
                    row, cell, in_cell, ..
                }) = stack.last_mut()
                {
                    if *in_cell {
                        row.push(cell.trim().to_string());
                        cell.clear();
                        *in_cell = false;
                    }
                }
            }
            Event::End(_) => {}
            Event::Text(t) => push_text(&mut stack, &t),
            Event::Code(t) => {
                push_text(&mut stack, "`");
                push_text(&mut stack, &t);
                push_text(&mut stack, "`");
            }
            Event::Html(t) | Event::InlineHtml(t) => push_text(&mut stack, t.trim()),
            Event::InlineMath(t) | Event::DisplayMath(t) => push_text(&mut stack, &t),
            Event::FootnoteReference(label) => {
                push_text(&mut stack, "[^");
                push_text(&mut stack, &label);
                push_text(&mut stack, "]");
            }
            Event::SoftBreak | Event::HardBreak => push_text(&mut stack, "\n"),
            Event::TaskListMarker(done) => {
                push_text(&mut stack, if done { "[x] " } else { "[ ] " });
            }
            Event::Rule => {}
        }
    }
    while !stack.is_empty() {
        pop_frame(&mut stack, &mut blocks);
    }
    blocks
}

fn level_no(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Append `s` to the innermost text holder, opening a paragraph when bare.
fn push_text(stack: &mut Vec<Frame>, s: &str) {
    match stack.last_mut() {
        Some(Frame::Heading { buf, .. })
        | Some(Frame::Para { buf })
        | Some(Frame::Code { buf, .. })
        | Some(Frame::Quote { buf }) => buf.push_str(s),
        Some(Frame::List { item, .. }) => item.push_str(s),
        Some(Frame::Table { cell, in_cell, .. }) => {
            if *in_cell {
                cell.push_str(s);
            }
        }
        None => {
            stack.push(Frame::Para { buf: s.to_string() });
        }
    }
}

/// Newline separator inside transparent containers (loose list items, ...).
fn push_sep(stack: &mut Vec<Frame>) {
    match stack.last_mut() {
        Some(Frame::List { item, .. }) if !item.is_empty() => item.push('\n'),
        Some(Frame::Quote { buf }) if !buf.is_empty() => buf.push('\n'),
        _ => {}
    }
}

/// Close the top frame; nested frames fold their text into the parent.
fn pop_frame(stack: &mut Vec<Frame>, blocks: &mut Vec<Block>) {
    let Some(frame) = stack.pop() else { return };
    match frame {
        Frame::Heading { level, buf } => emit(
            stack,
            blocks,
            Block::Heading {
                level,
                text: buf.trim().to_string(),
            },
        ),
        Frame::Para { buf } => {
            let text = buf.trim().to_string();
            if !text.is_empty() {
                emit(stack, blocks, Block::Para { text });
            }
        }
        Frame::Code { lang, buf } => {
            let code = buf.trim_end().to_string();
            if lang == "mermaid" {
                emit(
                    stack,
                    blocks,
                    Block::Mermaid {
                        source: code.trim().to_string(),
                    },
                );
            } else {
                emit(stack, blocks, Block::Code { lang, code });
            }
        }
        Frame::Quote { buf } => {
            let text = buf.trim().to_string();
            if !text.is_empty() {
                emit(stack, blocks, Block::Quote { text });
            }
        }
        Frame::List { mut items, item } => {
            flush_item(&mut items, item);
            if !items.is_empty() {
                emit(stack, blocks, Block::List { items });
            }
        }
        Frame::Table { head, rows, .. } => {
            if !head.is_empty() || !rows.is_empty() {
                emit(stack, blocks, Block::Table { head, rows });
            }
        }
    }
}

/// `End(Item)` (and list close) move finished item text into the list.
fn flush_item(items: &mut Vec<String>, item: String) {
    let text = item.trim().to_string();
    if !text.is_empty() {
        items.push(text);
    }
}

fn emit(stack: &mut Vec<Frame>, blocks: &mut Vec<Block>, block: Block) {
    match stack.last_mut() {
        None => blocks.push(block),
        Some(parent) => fold_into(parent, block),
    }
}

/// Nested block text folds into the enclosing frame as plain lines.
fn fold_into(parent: &mut Frame, block: Block) {
    let line = match block {
        Block::Heading { text, .. } | Block::Para { text } | Block::Quote { text } => text,
        Block::Code { code, .. } => code,
        Block::Mermaid { source } => source,
        Block::List { items } => items.join("\n"),
        Block::Table { head, rows } => {
            let mut lines = if head.is_empty() {
                Vec::new()
            } else {
                vec![head.join(" | ")]
            };
            lines.extend(rows.iter().map(|r| r.join(" | ")));
            lines.join("\n")
        }
    };
    match parent {
        Frame::Heading { buf, .. }
        | Frame::Para { buf }
        | Frame::Code { buf, .. }
        | Frame::Quote { buf } => {
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf.push_str(&line);
        }
        Frame::List { item, .. } => {
            if !item.is_empty() {
                item.push('\n');
            }
            item.push_str(&line);
        }
        Frame::Table { cell, in_cell, .. } => {
            if *in_cell {
                cell.push_str(&line);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_and_para() {
        assert_eq!(
            parse_markdown("# Hi\n\nhello\n"),
            vec![
                Block::Heading {
                    level: 1,
                    text: "Hi".to_string()
                },
                Block::Para {
                    text: "hello".to_string()
                },
            ]
        );
    }

    #[test]
    fn fenced_code_keeps_lang() {
        assert_eq!(
            parse_markdown("```rs\nlet x = 1;\n```\n"),
            vec![Block::Code {
                lang: "rs".to_string(),
                code: "let x = 1;".to_string()
            }]
        );
    }

    #[test]
    fn mermaid_fence_becomes_diagram() {
        assert_eq!(
            parse_markdown("```mermaid\nflowchart TD\n```\n"),
            vec![Block::Mermaid {
                source: "flowchart TD".to_string()
            }]
        );
    }

    #[test]
    fn bullets_become_list() {
        assert_eq!(
            parse_markdown("- a\n- b\n"),
            vec![Block::List {
                items: vec!["a".to_string(), "b".to_string()]
            }]
        );
    }

    #[test]
    fn quote_folds_paragraph() {
        assert_eq!(
            parse_markdown("> hi there\n"),
            vec![Block::Quote {
                text: "hi there".to_string()
            }]
        );
    }

    #[test]
    fn pipe_table_head_and_rows() {
        assert_eq!(
            parse_markdown("| a | b |\n|---|---|\n| 1 | 2 |\n"),
            vec![Block::Table {
                head: vec!["a".to_string(), "b".to_string()],
                rows: vec![vec!["1".to_string(), "2".to_string()]],
            }]
        );
    }

    #[test]
    fn blank_input_yields_no_blocks() {
        assert!(parse_markdown("   \n").is_empty());
    }

    #[test]
    fn inline_markup_folds_to_text() {
        assert_eq!(
            parse_markdown("see **bold** and `code`\n"),
            vec![Block::Para {
                text: "see bold and `code`".to_string()
            }]
        );
    }
}
