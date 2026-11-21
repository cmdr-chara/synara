//! Clean-room port of `packages/contracts/src/browserAutomationCssSelector.ts`,
//! written from recorded behavior only: a CSS-selector-subset validator over
//! UTF-16 code units.

#![forbid(unsafe_code)]

const BACKSLASH: u16 = 0x5C;
const SPACE: u16 = 0x20;
const COMMA: u16 = 0x2C;
const GT: u16 = 0x3E;
const PLUS: u16 = 0x2B;
const TILDE: u16 = 0x7E;
const HASH: u16 = 0x23;
const DOT: u16 = 0x2E;
const LBRACKET: u16 = 0x5B;
const RBRACKET: u16 = 0x5D;
const EQUALS: u16 = 0x3D;
const DQUOTE: u16 = 0x22;
const SQUOTE: u16 = 0x27;

fn is_name_start(u: u16) -> bool {
    u == b'-' as u16
        || u == b'_' as u16
        || u >= 0x80
        || (0x41..=0x5A).contains(&u)
        || (0x61..=0x7A).contains(&u)
}

fn is_name_char(u: u16) -> bool {
    is_name_start(u) || (0x30..=0x39).contains(&u)
}

fn is_hex(u: u16) -> bool {
    (0x30..=0x39).contains(&u) || (0x61..=0x66).contains(&u) || (0x41..=0x46).contains(&u)
}

fn hex_val(u: u16) -> u32 {
    match u {
        0x30..=0x39 => (u - 0x30) as u32,
        0x41..=0x46 => (u - 0x41 + 10) as u32,
        _ => (u - 0x61 + 10) as u32,
    }
}

struct Scanner<'a> {
    u: &'a [u16],
    i: usize,
}

impl<'a> Scanner<'a> {
    fn peek(&self) -> Option<u16> {
        self.u.get(self.i).copied()
    }

    fn skip_spaces(&mut self) {
        while self.peek() == Some(SPACE) {
            self.i += 1;
        }
    }

    /// `\` + hex{1,6} codepoint escape (optional single-space terminator) or
    /// `\` + any non-hex literal. Fails on nothing to escape or a bad codepoint
    /// (control, DEL, surrogate, > 0x10FFFF).
    fn escape(&mut self) -> bool {
        let Some(c) = self.u.get(self.i + 1).copied() else {
            return false;
        };
        if !is_hex(c) {
            self.i += 2;
            return true;
        }
        let mut j = self.i + 1;
        let mut cp: u32 = 0;
        for _ in 0..6 {
            match self.u.get(j).copied() {
                Some(h) if is_hex(h) => {
                    cp = cp * 16 + hex_val(h);
                    j += 1;
                }
                _ => break,
            }
        }
        if !(0x20..=0x10FFFF).contains(&cp) || cp == 0x7F || (0xD800..=0xDFFF).contains(&cp) {
            return false;
        }
        self.i = j;
        if self.peek() == Some(SPACE) {
            self.i += 1;
        }
        true
    }

    /// An identifier: `[a-zA-Z_-\x80-\uFFFF]` or an escape to start, then the
    /// same plus digits. Used for `.`/`#` names, attribute names and values.
    fn ident(&mut self) -> bool {
        match self.peek() {
            Some(BACKSLASH) => {
                if !self.escape() {
                    return false;
                }
            }
            Some(c) if is_name_start(c) => self.i += 1,
            _ => return false,
        }
        loop {
            match self.peek() {
                Some(c) if is_name_char(c) => self.i += 1,
                Some(BACKSLASH) => {
                    if !self.escape() {
                        break;
                    }
                }
                _ => break,
            }
        }
        true
    }

    /// An element name: like `ident` but must start with a literal char.
    fn element(&mut self) -> bool {
        match self.peek() {
            Some(c) if is_name_start(c) => self.i += 1,
            _ => return false,
        }
        loop {
            match self.peek() {
                Some(c) if is_name_char(c) => self.i += 1,
                Some(BACKSLASH) => {
                    if !self.escape() {
                        break;
                    }
                }
                _ => break,
            }
        }
        true
    }

    /// `\d+(\.\d+)?` — a bare number is a compound on its own (no subclasses).
    fn number(&mut self) -> bool {
        while self.peek().is_some_and(|c| (0x30u16..=0x39).contains(&c)) {
            self.i += 1;
        }
        if self.peek() == Some(DOT)
            && self
                .u
                .get(self.i + 1)
                .is_some_and(|c| (0x30u16..=0x39).contains(c))
        {
            self.i += 1;
            while self.peek().is_some_and(|c| (0x30u16..=0x39).contains(&c)) {
                self.i += 1;
            }
        }
        true
    }

    fn quoted(&mut self, quote: u16) -> bool {
        self.i += 1;
        while let Some(c) = self.peek() {
            if c == quote {
                self.i += 1;
                return true;
            }
            self.i += 1;
        }
        false
    }

    fn attr(&mut self) -> bool {
        self.i += 1; // consume '['
        self.skip_spaces();
        if !self.ident() {
            return false;
        }
        self.skip_spaces();
        if self.peek() == Some(RBRACKET) {
            self.i += 1;
            return true;
        }
        if matches!(
            self.peek(),
            Some(TILDE) | Some(0x7C) | Some(0x5E) | Some(0x24) | Some(0x2A)
        ) {
            self.i += 1;
        }
        if self.peek() != Some(EQUALS) {
            return false;
        }
        self.i += 1;
        self.skip_spaces();
        match self.peek() {
            Some(DQUOTE) | Some(SQUOTE) => {
                if !self.quoted(self.peek().unwrap()) {
                    return false;
                }
            }
            _ => {
                let mut took = false;
                loop {
                    match self.peek() {
                        Some(c) if is_name_char(c) => {
                            self.i += 1;
                            took = true;
                        }
                        Some(BACKSLASH) => {
                            if !self.escape() {
                                return false;
                            }
                            took = true;
                        }
                        _ => break,
                    }
                }
                if !took {
                    return false;
                }
            }
        }
        self.skip_spaces();
        if matches!(
            self.peek(),
            Some(0x69) | Some(0x49) | Some(0x73) | Some(0x53)
        ) {
            self.i += 1;
            self.skip_spaces();
        }
        if self.peek() == Some(RBRACKET) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn compound(&mut self) -> bool {
        match self.peek() {
            Some(c) if (0x30u16..=0x39).contains(&c) => return self.number(),
            _ => {}
        }
        let mut any = false;
        if self.peek().is_some_and(is_name_start) {
            let _ = self.element();
            any = true;
        }
        loop {
            match self.peek() {
                Some(HASH) | Some(DOT) => {
                    self.i += 1;
                    if !self.ident() {
                        return false;
                    }
                    any = true;
                }
                Some(LBRACKET) => {
                    if !self.attr() {
                        return false;
                    }
                    any = true;
                }
                _ => break,
            }
        }
        any
    }
}

/// Port of `isBrowserCssSelector`: a subset CSS selector — `compound` units
/// joined by ` `, `,`, `>`, `+`, `~` combinators, where a compound is a
/// number, an element name, and/or any run of `#id`, `.class`, `[attr]`.
pub fn is_browser_css_selector(value: String) -> Result<bool, String> {
    let units: Vec<u16> = value.encode_utf16().collect();
    if units.iter().any(|&c| c < 0x20 || c == 0x7F) {
        return Ok(false);
    }
    let mut p = Scanner { u: &units, i: 0 };
    p.skip_spaces();
    if !p.compound() {
        return Ok(false);
    }
    loop {
        let j = p.i;
        p.skip_spaces();
        if p.i == units.len() {
            return Ok(true);
        }
        match p.peek() {
            Some(COMMA) | Some(GT) | Some(PLUS) | Some(TILDE) => {
                p.i += 1;
                p.skip_spaces();
                if !p.compound() {
                    return Ok(false);
                }
            }
            _ if p.i > j => {
                if !p.compound() {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        }
    }
}
