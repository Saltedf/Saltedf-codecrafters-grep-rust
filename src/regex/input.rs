use std::ops::Index;

pub struct Text<'t> {
    text: &'t str,
}

impl<'t> Text<'t> {
    pub fn new(text: &'t str) -> Self {
        Self { text }
    }
    pub fn text(&self) -> &str {
        self.text
    }
    /// 左闭右开
    pub fn slice(&self, start: usize, end: usize) -> &str {
        self.text.get(start..end).unwrap_or("")
    }

    pub fn matchwith_at(&self, index: usize, ch: &char) -> bool {
        self.char_at(index).is_some_and(|c| c == *ch)
    }

    pub fn char_at(&self, index: usize) -> Option<char> {
        self.text.get(index..)?.chars().next()
    }
    pub fn is_end(&self, index: usize) -> bool {
        self.text.get(index..).is_none_or(|s| s.is_empty())
    }

    pub fn next_cursor_unsafe(&self, current_cursor: usize) -> usize {
        let slice = self.text.get(current_cursor..).unwrap();
        // eprintln!(">>> {}", slice);
        let current_len = slice.chars().next().unwrap().len_utf8();
        current_cursor + current_len
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Error;

    use crate::regex::input::*;

    #[test]
    fn test_char_at() {
        let s = "";
        let text = Text::new(s);
        let a = text.char_at(0);
        eprintln!("{:?}", a);
    }
}
