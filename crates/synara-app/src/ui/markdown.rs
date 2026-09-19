//! CommonMark and GFM text rendered by GPUI. No HTML or remote media execution.
use super::*;
use gpui::{FontStyle, FontWeight, HighlightStyle, InteractiveText, StyledText};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

#[derive(Clone, Default, Debug)]
struct Span {
    range: Range<usize>,
    bold: bool,
    italic: bool,
    code: bool,
    link: Option<String>,
}
#[derive(Default, Debug)]
struct Table {
    alignments: Vec<Alignment>,
    rows: Vec<Vec<Block>>,
}
#[derive(Default, Debug)]
struct Block {
    text: String,
    spans: Vec<Span>,
    marker: Option<String>,
    task: Option<bool>,
    indent: usize,
    heading: Option<u8>,
    code: bool,
    language: Option<String>,
    quote: bool,
    rule: bool,
    table: Option<Table>,
}

fn parse(source: &str) -> Vec<Block> {
    let mut blocks = vec![];
    let mut block = Block::default();
    let mut lists: Vec<Option<u64>> = vec![];
    let mut marker = None;
    let (mut bold, mut italic, mut quotes) = (0usize, 0usize, 0usize);
    let mut link = None;
    let mut table: Option<Table> = None;
    let mut row = Vec::new();
    let flush = |block: &mut Block, blocks: &mut Vec<Block>| {
        if !block.text.is_empty() || block.rule || block.code || block.table.is_some() {
            blocks.push(std::mem::take(block));
        }
    };
    let options = Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS;
    for event in Parser::new_ext(source, options) {
        let mut content = None;
        let mut code = false;
        match event {
            Event::Start(Tag::Paragraph) => flush(&mut block, &mut blocks),
            Event::End(TagEnd::Paragraph) => flush(&mut block, &mut blocks),
            Event::Start(Tag::Heading { level, .. }) => {
                flush(&mut block, &mut blocks);
                block.heading = Some(level as u8);
            }
            Event::End(TagEnd::Heading(_)) => flush(&mut block, &mut blocks),
            Event::Start(Tag::CodeBlock(kind)) => {
                flush(&mut block, &mut blocks);
                block.code = true;
                block.marker = marker.take();
                block.indent = lists
                    .len()
                    .saturating_sub(usize::from(block.marker.is_some()));
                block.quote = quotes > 0;
                if let CodeBlockKind::Fenced(info) = kind {
                    block.language = info.split_whitespace().next().map(str::to_owned);
                }
            }
            Event::End(TagEnd::CodeBlock) => flush(&mut block, &mut blocks),
            Event::Start(Tag::Table(alignments)) => {
                flush(&mut block, &mut blocks);
                table = Some(Table {
                    alignments,
                    rows: Vec::new(),
                });
            }
            Event::Start(Tag::TableCell) => block = Block::default(),
            Event::End(TagEnd::TableCell) => row.push(std::mem::take(&mut block)),
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                if let Some(table) = &mut table {
                    table.rows.push(std::mem::take(&mut row));
                }
            }
            Event::End(TagEnd::Table) => {
                block.table = table.take();
                block.marker = marker.take();
                block.indent = lists
                    .len()
                    .saturating_sub(usize::from(block.marker.is_some()));
                block.quote = quotes > 0;
                flush(&mut block, &mut blocks);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush(&mut block, &mut blocks);
                quotes += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush(&mut block, &mut blocks);
                quotes = quotes.saturating_sub(1);
            }
            Event::Start(Tag::List(start)) => {
                flush(&mut block, &mut blocks);
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                flush(&mut block, &mut blocks);
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                flush(&mut block, &mut blocks);
                marker = Some(match lists.last_mut() {
                    Some(Some(number)) => {
                        let value = format!("{number}.");
                        *number += 1;
                        value
                    }
                    _ => "•".into(),
                });
            }
            Event::End(TagEnd::Item) => {
                flush(&mut block, &mut blocks);
                marker = None;
            }
            Event::TaskListMarker(checked) => block.task = Some(checked),
            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold = bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic = italic.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => link = None,
            Event::Text(value) | Event::Html(value) | Event::InlineHtml(value) => {
                content = Some(value.to_string())
            }
            Event::Code(value) => {
                content = Some(value.to_string());
                code = true;
            }
            Event::SoftBreak => content = Some(" ".into()),
            Event::HardBreak => content = Some("\n".into()),
            Event::Rule => {
                flush(&mut block, &mut blocks);
                block.rule = true;
                flush(&mut block, &mut blocks);
            }
            _ => {}
        }
        if let Some(content) = content {
            if block.text.is_empty() && table.is_none() && !block.code {
                block.marker = marker.take();
                block.indent = lists
                    .len()
                    .saturating_sub(usize::from(block.marker.is_some()));
                block.quote = quotes > 0;
            }
            let start = block.text.len();
            block.text.push_str(&content);
            block.spans.push(Span {
                range: start..block.text.len(),
                bold: bold > 0,
                italic: italic > 0,
                code,
                link: link.clone(),
            });
        }
    }
    flush(&mut block, &mut blocks);
    blocks
}

fn render_inline(block: &Block, id: SharedString) -> gpui::AnyElement {
    let links: Vec<_> = block
        .spans
        .iter()
        .filter_map(|span| {
            span.link
                .as_ref()
                .filter(|url| synara_agent::validate_web_url(url).is_ok())
                .map(|url| (span.range.clone(), url.clone()))
        })
        .collect();
    let highlights = block
        .spans
        .iter()
        .map(|span| {
            (
                span.range.clone(),
                HighlightStyle {
                    font_weight: span.bold.then_some(FontWeight::SEMIBOLD),
                    font_style: span.italic.then_some(FontStyle::Italic),
                    color: span
                        .link
                        .as_ref()
                        .filter(|url| synara_agent::validate_web_url(url).is_ok())
                        .map(|_| rgb(palette().focus).into()),
                    background_color: span.code.then(|| rgb(palette().overlay).into()),
                    ..Default::default()
                },
            )
        })
        .collect::<Vec<_>>();
    let families = block
        .spans
        .iter()
        .filter(|span| span.code)
        .map(|span| (span.range.clone(), code_font()))
        .collect::<Vec<_>>();
    let text = StyledText::new(block.text.clone())
        .with_highlights(highlights)
        .with_font_family_overrides(families);
    InteractiveText::new(id, text)
        .on_click(
            links.iter().map(|(range, _)| range.clone()).collect(),
            move |index, _, cx| {
                if let Some((_, url)) = links.get(index) {
                    cx.open_url(url);
                }
            },
        )
        .into_any_element()
}

fn render_table(table: Table, id: &str) -> gpui::AnyElement {
    let columns = table.alignments.len().max(1);
    let alignments = table.alignments;
    let id = id.to_owned();
    div()
        .id(SharedString::from(format!("{id}-scroll")))
        .relative()
        .w_full()
        .min_w_0()
        .overflow_x_scroll()
        .child(layout_probe("markdown-table"))
        .child(
            div()
                .w_full()
                .min_w(px(columns as f32 * 112.))
                .rounded_md()
                .border_1()
                .border_color(rgb(palette().border))
                .children(table.rows.into_iter().enumerate().map(move |(row, cells)| {
                    let id = id.clone();
                    let alignments = alignments.clone();
                    div()
                        .flex()
                        .w_full()
                        .when(row == 0, |el| {
                            el.bg(rgb(palette().overlay))
                                .font_weight(FontWeight::SEMIBOLD)
                        })
                        .when(row > 0, |el| {
                            el.border_t_1().border_color(rgb(palette().border))
                        })
                        .children(cells.into_iter().enumerate().map(move |(column, cell)| {
                            let text = render_inline(
                                &cell,
                                SharedString::from(format!("{id}-{row}-{column}")),
                            );
                            div()
                                .w(gpui::relative(1. / columns as f32))
                                .flex_shrink_0()
                                .min_w_0()
                                .px_3()
                                .py_2()
                                .text_size(px(13.))
                                .when(column > 0, |el| {
                                    el.border_l_1().border_color(rgb(palette().border))
                                })
                                .text_align(match alignments.get(column) {
                                    Some(Alignment::Center) => gpui::TextAlign::Center,
                                    Some(Alignment::Right) => gpui::TextAlign::Right,
                                    _ => gpui::TextAlign::Left,
                                })
                                .child(text)
                        }))
                })),
        )
        .into_any_element()
}

fn render_code(block: &Block, id: &str) -> gpui::AnyElement {
    let text = render_inline(block, SharedString::from(format!("{id}-text")));
    let code = block.text.clone();
    div()
        .min_w_0()
        .rounded_lg()
        .bg(rgb(palette().overlay))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .py_1()
                .border_b_1()
                .border_color(rgb(palette().border))
                .text_size(px(11.))
                .text_color(rgb(palette().muted))
                .child(block.language.clone().unwrap_or_else(|| "Code".into()))
                .child(
                    button_shell(
                        SharedString::from(format!("{id}-copy")),
                        "Copy code",
                        false,
                    )
                    .relative()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(11.))
                    .px_2()
                    .child(icon(Glyph::Copy).size(px(13.)))
                    .child("Copy")
                    .child(layout_probe("markdown-copy"))
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(code.clone()));
                        cx.stop_propagation();
                    }),
                ),
        )
        .child(
            div()
                .p_3()
                .font_family(code_font())
                .text_size(px(13.))
                .child(text),
        )
        .into_any_element()
}

pub fn render(source: &str, message_id: &str) -> gpui::AnyElement {
    let blocks = parse(source);
    let block_count = blocks.len();
    let list_ends: Vec<_> = blocks
        .iter()
        .enumerate()
        .map(|(i, block)| {
            block.marker.is_some() && blocks.get(i + 1).is_some_and(|next| next.marker.is_none())
        })
        .collect();
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .children(blocks.into_iter().enumerate().map(|(index, mut block)| {
            let id = SharedString::from(format!("markdown-{message_id}-{index}"));
            let content = if let Some(table) = block.table.take() {
                render_table(table, &id)
            } else if block.code {
                render_code(&block, &id)
            } else {
                render_inline(&block, id.clone())
            };
            div()
                .id(id.clone())
                .min_w_0()
                .flex()
                .gap(px(8.))
                .pl(px(block.indent as f32 * 18.))
                .mb(px(
                    if index + 1 == block_count || (block.marker.is_some() && !list_ends[index]) {
                        4.
                    } else {
                        12.
                    },
                ))
                .when(block.quote, |el| {
                    el.border_l_2()
                        .border_color(rgb(palette().border))
                        .pl_3()
                        .text_color(rgb(palette().muted))
                })
                .children(block.marker.map(|marker| {
                    if let Some(checked) = block.task {
                        div()
                            .id(SharedString::from(format!("{id}-task")))
                            .aria_label(if checked { "Completed" } else { "Not completed" })
                            .flex_shrink_0()
                            .mt(px(4.))
                            .size(px(14.))
                            .rounded(px(3.))
                            .border_1()
                            .border_color(rgb(palette().muted))
                            .children(checked.then(|| icon(Glyph::Check).size(px(12.))))
                            .into_any_element()
                    } else {
                        div()
                            .flex_shrink_0()
                            .min_w(px(12.))
                            .child(marker)
                            .into_any_element()
                    }
                }))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .when_some(block.heading, |el, level| {
                            el.text_size(px(match level {
                                1 => 24.,
                                2 => 21.,
                                _ => 17.,
                            }))
                            .font_weight(FontWeight::SEMIBOLD)
                        })
                        .when(block.rule, |el| el.h(px(1.)).bg(rgb(palette().border)))
                        .child(content),
                )
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_keeps_unicode_ranges_and_nested_styles_in_list_items() {
        let blocks = parse(
            "Completato.\n\n- **Caffè [PR](https://github.com/a/b/pull/1)** — `ChatView`\n- Secondo",
        );
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[1].marker.as_deref(), Some("•"));
        assert_eq!(blocks[1].text, "Caffè PR — ChatView");
        let link = blocks[1]
            .spans
            .iter()
            .find(|span| span.link.is_some())
            .unwrap();
        assert!(link.bold);
        assert_eq!(&blocks[1].text[link.range.clone()], "PR");
        assert!(blocks[1].spans.iter().any(|span| span.code));
    }
    #[test]
    fn code_and_html_remain_inert_text() {
        let blocks = parse("```sh\nrm example\n```\n\n<script>alert(1)</script>");
        assert!(blocks[0].code);
        assert_eq!(blocks[0].language.as_deref(), Some("sh"));
        assert_eq!(blocks[0].text, "rm example\n");
        assert!(blocks[1].text.contains("<script>"));
        assert!(
            blocks
                .iter()
                .flat_map(|b| &b.spans)
                .all(|s| s.link.is_none())
        );
    }
    #[test]
    fn tables_preserve_alignment_empty_cells_unicode_and_inline_styles() {
        let blocks = parse(
            "Before\n\n| Name | Count | Notes |\n| :--- | ---: | :---: |\n| **Caffè** | `2` | |\n| [Docs](https://example.com) | 3 | Ready |\n\nAfter",
        );
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].text, "Before");
        assert_eq!(blocks[2].text, "After");
        let table = blocks[1].table.as_ref().unwrap();
        assert_eq!(
            table.alignments,
            vec![Alignment::Left, Alignment::Right, Alignment::Center]
        );
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.rows[1][0].text, "Caffè");
        assert!(table.rows[1][0].spans[0].bold);
        assert!(table.rows[1][1].spans[0].code);
        assert!(table.rows[1][2].text.is_empty());
        assert_eq!(
            table.rows[2][0].spans[0].link.as_deref(),
            Some("https://example.com")
        );
        for row in &table.rows {
            assert_eq!(row.len(), 3);
            for cell in row {
                for span in &cell.spans {
                    assert!(cell.text.get(span.range.clone()).is_some());
                }
            }
        }
    }
    #[test]
    fn task_lists_keep_read_only_completion_state_and_nested_items() {
        let blocks = parse("- [x] Done\n- [ ] Pending\n  - [x] Child\n\nNormal");
        assert_eq!(blocks[0].task, Some(true));
        assert_eq!(blocks[1].task, Some(false));
        assert_eq!(blocks[2].task, Some(true));
        assert_eq!(blocks[2].indent, 1);
        assert_eq!(blocks[3].task, None);
        assert_eq!(blocks[3].text, "Normal");
    }
    #[test]
    fn table_syntax_inside_code_is_not_interpreted() {
        let source = "| Name |\n| --- |\n| Value |\n";
        let blocks = parse(&format!("```markdown extra-info\n{source}```"));
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language.as_deref(), Some("markdown"));
        assert_eq!(blocks[0].text, source);
        assert!(blocks[0].table.is_none());
    }
    #[test]
    fn empty_fence_does_not_turn_the_next_paragraph_into_code() {
        let blocks = parse("```rust\n```\n\nNormal");
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].code);
        assert!(blocks[0].text.is_empty());
        assert!(!blocks[1].code);
        assert_eq!(blocks[1].text, "Normal");
    }
    #[test]
    fn consecutive_tables_do_not_share_rows_or_styles() {
        let blocks = parse("| A |\n| --- |\n| **one** |\n\nBreak\n\n| B |\n| --- |\n| two |");
        assert_eq!(blocks.len(), 3);
        let first = blocks[0].table.as_ref().unwrap();
        let second = blocks[2].table.as_ref().unwrap();
        assert_eq!(first.rows.len(), 2);
        assert_eq!(second.rows.len(), 2);
        assert!(first.rows[1][0].spans[0].bold);
        assert!(!second.rows[1][0].spans[0].bold);
        assert_eq!(second.rows[1][0].text, "two");
    }
}
