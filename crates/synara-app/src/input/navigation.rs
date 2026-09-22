//! Explicit editor navigation and replacements share the native undo/IME owner.
use super::*;

fn line_range(text: &str, range: std::ops::Range<usize>) -> (usize, usize) {
    let start = range.start.min(text.len());
    let last = if range.end > start {
        range.end.min(text.len()).saturating_sub(1)
    } else {
        start
    };
    let line = |offset| {
        text.as_bytes()[..offset]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
            + 1
    };
    (line(start), line(last))
}

impl TextEntry {
    pub fn selected_line_range(&self) -> (usize, usize) {
        line_range(self.text(), self.buffer.selection())
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        let text = &self.buffer.text()[..self.caret()];
        (
            text.bytes().filter(|b| *b == b'\n').count() + 1,
            text.rsplit('\n').next().unwrap_or_default().chars().count() + 1,
        )
    }

    pub fn go_to_line(&mut self, line: usize, column: usize, cx: &mut Context<Self>) -> bool {
        if line == 0 || column == 0 || self.is_composing() {
            return false;
        }
        let text = self.buffer.text();
        let start = if line == 1 {
            Some(0)
        } else {
            text.match_indices('\n')
                .nth(line - 2)
                .map(|(index, _)| index + 1)
        };
        let Some(start) = start else { return false };
        let remaining = text[start..]
            .split('\n')
            .next()
            .unwrap_or_default()
            .trim_end_matches('\r');
        let offset = remaining
            .char_indices()
            .nth(column - 1)
            .map_or(remaining.len(), |(index, _)| index);
        self.select_to(start + offset, false, cx);
        true
    }

    pub fn find_literal(&mut self, query: &str, backwards: bool, cx: &mut Context<Self>) -> bool {
        if query.is_empty() || self.is_composing() {
            return false;
        }
        let text = self.buffer.text();
        let selection = self.buffer.selection();
        let index = if backwards {
            text[..selection.start]
                .rfind(query)
                .or_else(|| text.rfind(query))
        } else {
            text[selection.end..]
                .find(query)
                .map(|index| selection.end + index)
                .or_else(|| text.find(query))
        };
        let Some(index) = index else { return false };
        if self.buffer.select(index..index + query.len()).is_err() {
            return false;
        }
        self.anchor = None;
        self.reversed = false;
        self.ensure_caret = true;
        cx.notify();
        true
    }

    pub fn replace_literal(
        &mut self,
        query: &str,
        replacement: &str,
        all: bool,
        cx: &mut Context<Self>,
    ) -> usize {
        if query.is_empty() || self.is_composing() {
            return 0;
        }
        if all {
            let count = self.text().match_indices(query).count();
            if count == 0 {
                return 0;
            }
            let length = self
                .text()
                .len()
                .saturating_sub(count.saturating_mul(query.len()))
                .saturating_add(count.saturating_mul(replacement.len()));
            if length > MAX_INPUT {
                self.error = Some("Replacement would exceed 1 MiB. Nothing was changed.".into());
                cx.notify();
                return 0;
            }
            let text = self.text().replace(query, replacement);
            self.edit(0..self.text().len(), &text, cx);
            count
        } else {
            if self.selected_text() != query && !self.find_literal(query, false, cx) {
                return 0;
            }
            if self
                .text()
                .len()
                .saturating_sub(query.len())
                .saturating_add(replacement.len())
                > MAX_INPUT
            {
                self.error = Some("Replacement would exceed 1 MiB. Nothing was changed.".into());
                cx.notify();
                return 0;
            }
            self.edit(self.buffer.selection(), replacement, cx);
            self.find_literal(query, false, cx);
            1
        }
    }
}

#[cfg(test)]
mod line_range_tests {
    use super::line_range;
    #[test]
    fn range_end_is_exclusive_and_caret_at_newline_is_predictable() {
        assert_eq!(line_range("a\nb\n", 0..2), (1, 1));
        assert_eq!(line_range("a\nb\n", 2..4), (2, 2));
        assert_eq!(line_range("a\nb\n", 0..4), (1, 2));
        assert_eq!(line_range("a\nb\n", 4..4), (3, 3));
        assert_eq!(line_range("β\r\nnext", 0..4), (1, 1));
        assert_eq!(line_range("", 0..0), (1, 1));
    }
}
