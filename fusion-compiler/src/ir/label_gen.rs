use crate::ast::parser::Counter;
use crate::ir::basic_block::Label;

pub struct LabelGen {
    counter: Counter,
}

impl LabelGen {
    pub fn new() -> Self {
        Self {
            counter: Counter::new(),
        }
    }

    fn next_id(&self) -> usize {
        let id = self.counter.get_value();
        self.counter.increment();
        id
    }

    pub fn next_label(&self) -> Label {
        Label {
            name: format!("L{}", self.next_id()),
        }
    }
}
