use crate::ast::ASTStmtId;
use crate::compilation::symbols::variable::VariableSymbol;
use crate::typings::Type;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FunctionModifier {
    External
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionSymbol {
    pub parameters: Vec<VariableSymbol>,
    pub body: Option<ASTStmtId>,
    pub return_type: Type,
    pub name: String,
    pub modifiers: Vec<FunctionModifier>,
}

impl FunctionSymbol {
    pub fn new(parameters: Vec<VariableSymbol>, body: Option<ASTStmtId>, return_type: Type, name: String, modifiers: Vec<FunctionModifier>) -> Self {
        FunctionSymbol {
            parameters,
            body,
            name,
            return_type,
            modifiers,
        }
    }

    pub fn is_external(&self) -> bool {
        self.modifiers.contains(&FunctionModifier::External)
    }
}
