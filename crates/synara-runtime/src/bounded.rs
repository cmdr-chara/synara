use std::collections::VecDeque;

/// A byte-limited tail. Total input length remains available even after truncation.
#[derive(Clone, Debug)]
pub struct BoundedBytes {
    bytes: VecDeque<u8>,
    capacity: usize,
    total: u64,
}
impl BoundedBytes {
    pub fn new(capacity: usize) -> Self {
        Self {
            bytes: VecDeque::new(),
            capacity,
            total: 0,
        }
    }
    pub fn push(&mut self, input: &[u8]) {
        self.total = self.total.saturating_add(input.len() as u64);
        let tail = &input[input.len().saturating_sub(self.capacity)..];
        let excess = self
            .bytes
            .len()
            .saturating_add(tail.len())
            .saturating_sub(self.capacity);
        self.bytes.drain(..excess);
        self.bytes.extend(tail);
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.iter().copied().collect()
    }
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes()).into_owned()
    }
    pub fn total_bytes(&self) -> u64 {
        self.total
    }
    pub fn truncated(&self) -> bool {
        self.total > self.bytes.len() as u64
    }
    pub fn clear(&mut self) {
        self.bytes.clear();
        self.total = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_only_tail() {
        let mut b = BoundedBytes::new(4);
        b.push(b"123");
        b.push(b"456");
        assert_eq!(b.bytes(), b"3456");
        assert_eq!(b.total_bytes(), 6);
        assert!(b.truncated());
        b.push(b"abcdefgh");
        assert_eq!(b.bytes(), b"efgh");
    }
    #[test]
    fn zero_capacity_is_supported() {
        let mut b = BoundedBytes::new(0);
        b.push(b"abc");
        assert!(b.bytes().is_empty());
    }
}
