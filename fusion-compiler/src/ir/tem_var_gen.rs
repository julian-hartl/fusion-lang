use crate::ast::parser::Counter;
use crate::compilation::symbols::variable::VariableSymbol;
use crate::typings::Type;

pub struct TempVarGen {
    counter: Counter,
}

impl TempVarGen {
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

    pub fn next_temp_var(&self, ty: Type) -> VariableSymbol {
        let name = format!("t{}", self.next_id());
        VariableSymbol::new(name, ty)
    }
}
