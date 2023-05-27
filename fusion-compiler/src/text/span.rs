use crate::modules::symbols::ModuleIdx;

#[derive(Debug, PartialEq, Clone)]
pub struct TextLocation {
    pub span: TextSpan,
    pub module_id: ModuleIdx,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TextSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
    // todo: remove this and replace with indexing of actual source text
    pub(crate) literal: String,
}

impl Default for TextSpan {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            literal: String::new(),
        }
    }
}

impl TextSpan {
    pub fn new(start: usize, end: usize, literal: String) -> Self {
        Self { start, end, literal }
    }

    pub fn merge(mut spans: Vec<&TextSpan>) -> TextSpan {
        assert!(spans.len() > 0, "Cannot merge empty span list");
        spans.sort_by(
            |a, b| a.start.cmp(&b.start)
        );

        let start = spans.first().unwrap().start;
        let end = spans.last().unwrap().end;

        let mut literal = String::new();
        for (index, span) in spans.iter().enumerate() {
            if index > 0 {
                let last = spans.get(index - 1).unwrap();
                let diff = span.start.checked_sub(last.end).unwrap_or(0);
                literal.push_str(&" ".repeat(diff));
            }
            literal.push_str(&span.literal);
        }

        TextSpan::new(start, end, literal)
    }

    pub fn length(&self) -> usize {
        self.end - self.start
    }
}
