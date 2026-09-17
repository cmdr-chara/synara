use std::ops::Range;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextBuffer {
    text: String,
    selection: Range<usize>,
    marked: Option<Range<usize>>,
    revision: u64,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum TextError {
    #[error("range is outside the buffer or splits a character")]
    InvalidRange,
}

impl TextBuffer {
    pub fn new(text: String) -> Self { let n = text.len(); Self { text, selection: n..n, marked: None, revision: 0 } }
    pub fn text(&self) -> &str { &self.text }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn selection(&self) -> Range<usize> { self.selection.clone() }
    pub fn marked(&self) -> Option<Range<usize>> { self.marked.clone() }
    pub fn select(&mut self, range: Range<usize>) -> Result<(), TextError> {
        self.validate(&range)?; self.selection = range; Ok(())
    }
    pub fn replace(&mut self, range: Range<usize>, text: &str) -> Result<(), TextError> {
        self.validate(&range)?;
        let end = range.start + text.len();
        self.text.replace_range(range, text);
        self.selection = end..end;
        self.marked = None;
        self.revision += 1;
        Ok(())
    }
    pub fn insert(&mut self, text: &str) -> Result<(), TextError> { self.replace(self.selection(), text) }
    pub fn select_all(&mut self) { self.selection = 0..self.text.len(); }
    pub fn clear(&mut self) { self.text.clear(); self.selection = 0..0; self.marked = None; self.revision += 1; }
    pub fn backspace(&mut self) -> Result<(), TextError> {
        let mut range = self.selection();
        if range.is_empty() {
            range.start = self.text[..range.start].char_indices().last().map_or(0, |(i, _)| i);
        }
        self.replace(range, "")
    }
    pub fn delete_forward(&mut self) -> Result<(), TextError> {
        let mut range = self.selection();
        if range.is_empty() && range.end < self.text.len() {
            range.end += self.text[range.end..].chars().next().map_or(0, char::len_utf8);
        }
        self.replace(range, "")
    }
    pub fn move_left(&mut self) {
        let index = if self.selection.is_empty() { self.text[..self.selection.start].char_indices().last().map_or(0, |(i, _)| i) } else { self.selection.start };
        self.selection = index..index; self.marked = None;
    }
    pub fn move_right(&mut self) {
        let mut index = self.selection.end;
        if self.selection.is_empty() { index += self.text[index..].chars().next().map_or(0, char::len_utf8); }
        self.selection = index..index; self.marked = None;
    }
    pub fn byte_to_utf16(&self, offset: usize) -> Result<usize, TextError> {
        self.validate(&(offset..offset))?; Ok(self.text[..offset].encode_utf16().count())
    }
    pub fn utf16_to_byte(&self, offset: usize) -> Result<usize, TextError> {
        let mut units = 0;
        for (byte, ch) in self.text.char_indices() {
            if units == offset { return Ok(byte); }
            units += ch.len_utf16();
            if units > offset { return Err(TextError::InvalidRange); }
        }
        if units == offset { Ok(self.text.len()) } else { Err(TextError::InvalidRange) }
    }
    pub fn utf16_range_to_bytes(&self, range: Range<usize>) -> Result<Range<usize>, TextError> {
        if range.start > range.end { return Err(TextError::InvalidRange); }
        Ok(self.utf16_to_byte(range.start)?..self.utf16_to_byte(range.end)?)
    }
    pub fn mark(&mut self, range: Range<usize>) -> Result<(), TextError> { self.validate(&range)?; self.marked = Some(range); Ok(()) }
    pub fn unmark(&mut self) { self.marked = None; }
    fn validate(&self, range: &Range<usize>) -> Result<(), TextError> {
        if range.start <= range.end && range.end <= self.text.len() && self.text.is_char_boundary(range.start) && self.text.is_char_boundary(range.end) { Ok(()) } else { Err(TextError::InvalidRange) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn unicode_editing_never_splits_utf8() {
        let mut b = TextBuffer::new("a😀æ".into());
        b.backspace().unwrap(); assert_eq!(b.text(), "a😀");
        b.move_left(); b.insert("中").unwrap(); assert_eq!(b.text(), "a中😀");
        assert_eq!(b.replace(2..3, "x"), Err(TextError::InvalidRange));
    }
    #[test] fn utf16_rejects_half_surrogates() {
        let b = TextBuffer::new("a😀z".into());
        assert_eq!(b.utf16_to_byte(2), Err(TextError::InvalidRange));
        assert_eq!(b.utf16_to_byte(3), Ok(5)); assert_eq!(b.byte_to_utf16(5), Ok(3));
    }
    #[test] fn selection_replacement_and_marking() {
        let mut b = TextBuffer::new("hello".into());
        b.select(1..4).unwrap(); b.insert("i").unwrap(); assert_eq!(b.text(), "hio");
        b.mark(1..2).unwrap(); b.insert("!").unwrap(); assert!(b.marked().is_none());
    }
}
