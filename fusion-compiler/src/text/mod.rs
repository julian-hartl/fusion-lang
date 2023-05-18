pub mod span;
pub mod io;

pub struct SourceText {
    pub text: String,
}

impl SourceText {
    pub fn new(text: String) -> Self {
        Self {
            text,
        }
    }

    pub fn line_index(&self, position: usize) -> Option<usize> {
        if position >= self.text.len() {
            return None;
        }
        return Some(self.text[..=position].lines().count() - 1);
    }

    pub fn get_line(&self, index: usize) -> &str {
        self.text.lines().nth(index).unwrap()
    }

    pub fn line_start(&self, index: usize) -> usize {
        self.text.lines().take(index).map(|line| line.len() + 1).sum()
    }
}
