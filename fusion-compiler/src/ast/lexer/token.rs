use std::fmt::{Display, Formatter};
use crate::ast::lexer::StringToken;
use crate::text::span::TextSpan;

#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    // Literals
    Number(i64),
    Character(Box<Token>, char, Box<Token>),
    String(Box<Token>, StringToken, Box<Token>),
    // Operators
    Plus,
    Minus,
    Asterisk,
    Slash,
    Equals,
    Percent,
    Ampersand,
    DoubleAmpersand,
    Pipe,
    Caret,
    DoubleAsterisk,
    Tilde,
    GreaterThan,
    LessThan,
    GreaterThanEquals,
    LessThanEquals,
    EqualsEquals,
    BangEquals,
    // Keywords
    Let,
    If,
    Else,
    True,
    False,
    While,
    Extern,
    Func,
    Return,
    As,
    Mut,
    Struct,
    Mod,
    // Separators
    LeftParen,
    RightParen,
    OpenBrace,
    CloseBrace,
    Comma,
    Colon,
    Arrow,
    DoubleQuote,
    SingleQuote,
    Dot,
    SemiColon,
    ColonColon,
    OpenBracket,
    CloseBracket,
    // Other
    Bad,
    Whitespace,
    Identifier,
    Newline,
    Eof,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Number(_) => write!(f, "Number"),
            TokenKind::Plus => write!(f, "Plus"),
            TokenKind::Minus => write!(f, "Minus"),
            TokenKind::Asterisk => write!(f, "Asterisk"),
            TokenKind::Percent => write!(f, "Percent"),
            TokenKind::Slash => write!(f, "Slash"),
            TokenKind::LeftParen => write!(f, "LeftParen"),
            TokenKind::RightParen => write!(f, "RightParen"),
            TokenKind::Bad => write!(f, "Bad"),
            TokenKind::Whitespace => write!(f, "Whitespace"),
            TokenKind::Eof => write!(f, "Eof"),
            TokenKind::Let => write!(f, "Let"),
            TokenKind::Identifier => write!(f, "Identifier"),
            TokenKind::Equals => write!(f, "Equals"),
            TokenKind::Ampersand => write!(f, "Ampersand"),
            TokenKind::Pipe => write!(f, "Pipe"),
            TokenKind::Caret => write!(f, "Caret"),
            TokenKind::DoubleAsterisk => write!(f, "DoubleAsterisk"),
            TokenKind::Tilde => write!(f, "Tilde"),
            TokenKind::If => write!(f, "If"),
            TokenKind::Else => write!(f, "Else"),
            TokenKind::GreaterThan => write!(f, ">"),
            TokenKind::LessThan => write!(f, "<"),
            TokenKind::GreaterThanEquals => write!(f, ">="),
            TokenKind::LessThanEquals => write!(f, "<="),
            TokenKind::EqualsEquals => write!(f, "=="),
            TokenKind::BangEquals => write!(f, "!="),
            TokenKind::OpenBrace => write!(f, "{{"),
            TokenKind::CloseBrace => write!(f, "}}"),
            TokenKind::True => write!(f, "True"),
            TokenKind::False => write!(f, "False"),
            TokenKind::While => write!(f, "While"),
            TokenKind::Func => write!(f, "Func"),
            TokenKind::Return => write!(f, "Return"),
            TokenKind::Comma => write!(f, "Comma"),
            TokenKind::Colon => write!(f, "Colon"),
            TokenKind::Arrow => write!(f, "Arrow"),
            TokenKind::Newline => write!(f, "Newline"),
            TokenKind::DoubleQuote => write!(f, "Quote"),
            TokenKind::Extern => write!(f, "Extern"),
            TokenKind::Dot => write!(f, "Dot"),
            TokenKind::SingleQuote => write!(f, "SingleQuote"),
            TokenKind::SemiColon => write!(f, "SemiColon"),
            TokenKind::As => write!(f, "As"),
            TokenKind::Mut => write!(f, "Mut"),
            TokenKind::Struct => write!(f, "Struct"),
            TokenKind::ColonColon => write!(f, "ColonColon"),
            TokenKind::Mod => write!(f, "Mod"),
            TokenKind::OpenBracket => write!(f, "OpenBracket"),
            TokenKind::CloseBracket => write!(f, "CloseBracket"),
            TokenKind::DoubleAmpersand => write!(f, "DoubleAmpersand"),
            TokenKind::Character(_, _, _) => write!(f, "Character"),
            TokenKind::String(_, _, _) => write!(f, "String"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub(crate) kind: TokenKind,
    pub(crate) span: TextSpan,
}

impl Token {
    pub fn new(kind: TokenKind, span: TextSpan) -> Self {
        Self { kind, span }
    }
}
