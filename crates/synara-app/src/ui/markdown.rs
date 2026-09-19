//! CommonMark text rendered by native GPUI elements. No HTML or remote media execution.
use super::*;
use gpui::{FontStyle, FontWeight, HighlightStyle, InteractiveText, StyledText};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
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
struct Block {
    text: String,
    spans: Vec<Span>,
    marker: Option<String>,
    indent: usize,
    heading: Option<u8>,
    code: bool,
    quote: bool,
    rule: bool,
}

fn parse(source: &str) -> Vec<Block> {
    let mut blocks = vec![];
    let mut block = Block::default();
    let mut lists: Vec<Option<u64>> = vec![];
    let mut marker = None;
    let (mut bold, mut italic, mut quotes) = (0usize, 0usize, 0usize);
    let mut link = None;
    let flush = |block: &mut Block, blocks: &mut Vec<Block>| {
        if !block.text.is_empty() || block.rule {
            blocks.push(std::mem::take(block));
        }
    };
    for event in Parser::new_ext(source, Options::empty()) {
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
            Event::Start(Tag::CodeBlock(_)) => {
                flush(&mut block, &mut blocks);
                block.code = true;
            }
            Event::End(TagEnd::CodeBlock) => flush(&mut block, &mut blocks),
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
            if block.text.is_empty() {
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
        .children(blocks.into_iter().enumerate().map(|(index, block)| {
            let id = SharedString::from(format!("markdown-{message_id}-{index}"));
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
                                .map(|_| rgb(crate::ui::palette().focus).into()),
                            background_color: span
                                .code
                                .then(|| rgb(crate::ui::palette().overlay).into()),
                            ..Default::default()
                        },
                    )
                })
                .collect::<Vec<_>>();
            let families = block
                .spans
                .iter()
                .filter(|span| span.code)
                .map(|span| (span.range.clone(), crate::ui::code_font()))
                .collect::<Vec<_>>();
            let text = StyledText::new(block.text)
                .with_highlights(highlights)
                .with_font_family_overrides(families);
            let text = InteractiveText::new(id.clone(), text).on_click(
                links.iter().map(|(range, _)| range.clone()).collect(),
                move |index, _, cx| {
                    if let Some((_, url)) = links.get(index) {
                        cx.open_url(url);
                    }
                },
            );
            div()
                .id(id)
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
                .children(
                    block
                        .marker
                        .map(|marker| div().flex_shrink_0().min_w(px(12.)).child(marker)),
                )
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
                        .when(block.code, |el| {
                            el.p_3()
                                .rounded_lg()
                                .bg(rgb(palette().overlay))
                                .font_family(crate::ui::code_font())
                                .text_size(px(13.))
                        })
                        .when(block.rule, |el| el.h(px(1.)).bg(rgb(palette().border)))
                        .child(text),
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
        assert!(blocks[1].text.contains("<script>"));
        assert!(
            blocks
                .iter()
                .flat_map(|b| &b.spans)
                .all(|s| s.link.is_none())
        );
    }
}
