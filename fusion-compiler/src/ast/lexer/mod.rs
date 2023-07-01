use std::fmt;
use std::fmt::{Display, Formatter};

use clap::builder::Str;

use token::{Token, TokenKind};

use crate::ast::lexer::stream::TokenStream;
use crate::text::SourceText;
use crate::text::span::TextSpan;

pub mod token;
pub mod stream;

pub struct Lexer<'a> {
    input: &'a SourceText,
    current_pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a SourceText) -> Self {
        Self { input, current_pos: 0 }
    }

    pub fn token_stream(&'a mut self) -> TokenStream<'a> {
        let mut stream = TokenStream::new(self);
        stream.prepare();
        stream
    }

    pub fn next_token(&mut self) -> Option<Token> {
        if self.current_pos == self.input.text.len() {
            let eof_char: char = '\0';
            return Some(Token::new(
                TokenKind::Eof,
                TextSpan::new(self.input.text.len() - 1, self.input.text.len(), eof_char.to_string()),
            ));
        }
        let c = self.current_char();
        c.map(|c| {
            let start = self.current_pos;
            let kind = if Self::is_number_start(&c) {
                let number: i64 = self.consume_number();
                TokenKind::Number(number)
            } else if Self::is_char_start(&c) {
                // todo: fix this
                let open_quote_span = self.get_current_char_span();
                let open_quote_kind = self.consume_punctuation();
                assert_eq!(open_quote_kind, TokenKind::SingleQuote);
                let open_quote = Token::new(open_quote_kind, open_quote_span);
                let character = self.consume_char()?;
                let close_quote_span = self.get_current_char_span();
                let close_quote_kind = self.consume_punctuation();
                assert_eq!(close_quote_kind, TokenKind::SingleQuote);
                let close_quote = Token::new(close_quote_kind, close_quote_span);
                TokenKind::Character(Box::new(open_quote), character, Box::new(close_quote))
            } else if Self::is_string_start(&c) {
                let open_quote_span = self.get_current_char_span();
                let open_quote_kind = self.consume_punctuation();
                assert_eq!(open_quote_kind, TokenKind::DoubleQuote);
                let open_quote = Token::new(open_quote_kind, open_quote_span);
                let string = self.consume_string();
                let close_quote_span = self.get_current_char_span();
                let close_quote = Token::new(TokenKind::DoubleQuote, close_quote_span);
                TokenKind::String(Box::new(open_quote), string, Box::new(close_quote))
            } else if Self::is_new_line(&c) {
                self.consume();
                TokenKind::Newline
            } else if Self::is_whitespace(&c) {
                self.consume();
                TokenKind::Whitespace
            } else if Self::is_identifier_start(&c) {
                let identifier = self.consume_identifier();
                match identifier.as_str() {
                    "let" => TokenKind::Let,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "while" => TokenKind::While,
                    "extern" => TokenKind::Extern,
                    "func" => TokenKind::Func,
                    "return" => TokenKind::Return,
                    "as" => TokenKind::As,
                    "mut" => TokenKind::Mut,
                    "struct" => TokenKind::Struct,
                    "mod" => TokenKind::Mod,
                    _ => TokenKind::Identifier,
                }
            } else {
                self.consume_punctuation()
            };

            let end = self.current_pos;
            let literal = self.input.text[start..end].to_string();
            let span = TextSpan::new(start, end, literal);
            Some(Token::new(kind, span))
        }).map(|t| t.unwrap_or_else(|| Token::new(TokenKind::Bad, self.get_current_char_span())))
    }

    fn get_current_char_span(&mut self) -> TextSpan {
        TextSpan::new(self.current_pos, self.current_pos + 1, self.current_char().unwrap().to_string())
    }

    fn is_new_line(&c: &char) -> bool {
        c.is_whitespace() && c != ' '
    }

    fn consume_punctuation(&mut self) -> TokenKind {
        let c = self.consume().unwrap();
        match c {
            '+' => TokenKind::Plus,
            '-' => self.lex_potential_double_char_operator('>', TokenKind::Minus, TokenKind::Arrow),
            '*' => {
                self.lex_potential_double_char_operator('*', TokenKind::Asterisk, TokenKind::DoubleAsterisk)
            }
            '%' => TokenKind::Percent,
            '/' => TokenKind::Slash,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '=' => {
                self.lex_potential_double_char_operator('=', TokenKind::Equals, TokenKind::EqualsEquals)
            }
            '&' => {
                self.lex_potential_double_char_operator('&', TokenKind::Ampersand, TokenKind::DoubleAmpersand)
            }
            '|' => TokenKind::Pipe,
            '^' => TokenKind::Caret,
            '~' => TokenKind::Tilde,
            '>' => {
                self.lex_potential_double_char_operator('=', TokenKind::GreaterThan, TokenKind::GreaterThanEquals)
            }
            '<' => {
                self.lex_potential_double_char_operator('=', TokenKind::LessThan, TokenKind::LessThanEquals)
            }
            '!' => {
                self.lex_potential_double_char_operator('=', TokenKind::Bad, TokenKind::BangEquals)
            }
            '{' => {
                TokenKind::OpenBrace
            }
            '}' => {
                TokenKind::CloseBrace
            }
            ',' => {
                TokenKind::Comma
            }
            ':' => {
                self.lex_potential_double_char_operator(':', TokenKind::Colon, TokenKind::ColonColon)
            }
            '"' => {
                TokenKind::DoubleQuote
            }
            '.' => {
                TokenKind::Dot
            }
            '\'' => {
                TokenKind::SingleQuote
            }
            ';' => {
                TokenKind::SemiColon
            }
            '[' => {
                TokenKind::OpenBracket
            }
            ']' => {
                TokenKind::CloseBracket
            }
            _ => TokenKind::Bad,
        }
    }

    fn lex_potential_double_char_operator(&mut self, expected: char, one_char_kind: TokenKind, double_char_kind: TokenKind) -> TokenKind {
        if let Some(next) = self.current_char() {
            if next == expected {
                self.consume();
                double_char_kind
            } else {
                one_char_kind
            }
        } else {
            one_char_kind
        }
    }

    fn is_number_start(c: &char) -> bool {
        c.is_ascii_digit()
    }

    fn is_char_start(c: &char) -> bool {
        c == &'\''
    }

    fn is_string_start(c: &char) -> bool {
        c == &'"'
    }

    fn is_identifier_start(c: &char) -> bool {
        c.is_alphabetic() || c == &'_'
    }

    fn is_identifier_continue(c: &char) -> bool {
        Self::is_identifier_start(c) || c.is_ascii_digit()
    }

    fn is_whitespace(c: &char) -> bool {
        c.is_whitespace()
    }

    fn current_char(&self) -> Option<char> {
        self.input.text.chars().nth(self.current_pos)
    }

    fn consume(&mut self) -> Option<char> {
        if self.current_pos >= self.input.text.len() {
            return None;
        }
        let c = self.current_char();
        self.current_pos += 1;

        c
    }

    fn consume_identifier(&mut self) -> String {
        let mut identifier = String::new();
        if let Some(c) = self.current_char() {
            if Self::is_identifier_start(&c) {
                self.consume().unwrap();
                identifier.push(c);
            }
        }
        while let Some(c) = self.current_char() {
            if Self::is_identifier_continue(&c) {
                self.consume().unwrap();
                identifier.push(c);
            } else {
                break;
            }
        }
        identifier
    }

    fn consume_number(&mut self) -> i64 {
        let mut number: i64 = 0;
        while let Some(c) = self.current_char() {
            if c.is_ascii_digit() {
                self.consume().unwrap();
                number = number * 10 + c.to_digit(10).unwrap() as i64;
            } else {
                break;
            }
        }
        number
    }

    fn consume_char(&mut self) -> Option<char> {
        let mut c = self.consume()?;
        let c = if c == '\\' {
            c = self.consume()?;
            // todo: do not unwrap
            EscapedCharacter::from_char(c).map(|c| c.as_char()).expect("Invalid escape sequence")
        } else {
            c
        };
        Some(c)
    }

    fn consume_string(&mut self) -> StringToken {
        let mut parts = Vec::new();
        let mut literal = String::new();
        while let Some(c) = self.consume() {
            if c == '"' {
                parts.push(StringTokenPart::Literal(literal));
                literal = String::new();
                break;
            } else if c == '\\' {
                if let Some(escape_sequence) = self.consume_escape_sequence() {
                    parts.push(StringTokenPart::Literal(literal));
                    parts.push(StringTokenPart::EscapeSequence(escape_sequence));
                    literal = String::new();
                } else {
                    literal.push('\\');
                }
            } else {
                literal.push(c);
            }
        }
        parts.push(StringTokenPart::Literal(literal));
        StringToken::new(parts)
    }

    fn consume_escape_sequence(&mut self) -> Option<EscapedCharacter> {
        let c = self.consume()?;
        EscapedCharacter::from_char(c).ok()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StringToken {
    pub parts: Vec<StringTokenPart>,
}

impl StringToken {
    pub fn new(parts: Vec<StringTokenPart>) -> Self {
        StringToken { parts }
    }

    pub fn to_raw_string(&self) -> String {
        let mut result = String::new();
        for part in &self.parts {
            match part {
                StringTokenPart::Literal(literal) => result.push_str(literal),
                StringTokenPart::EscapeSequence(escape_sequence) => result.push_str(&escape_sequence.as_raw_string().to_string())
            }
        }
        result
    }

}

impl Display for StringToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for part in &self.parts {
            match part {
                StringTokenPart::Literal(literal) => write!(f, "{}", literal)?,
                StringTokenPart::EscapeSequence(escape_sequence) => {
                    write!(f, "{}", escape_sequence.as_string())?
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscapedCharacter {
    Newline,
    CarriageReturn,
    Tab,
    Quote,
    Zero,
}

impl EscapedCharacter {
    pub fn from_char(c: char) -> fusion_compiler::Result<Self> {
        match c {
            'n' => Ok(EscapedCharacter::Newline),
            'r' => Ok(EscapedCharacter::CarriageReturn),
            't' => Ok(EscapedCharacter::Tab),
            '"' => Ok(EscapedCharacter::Quote),
            '0' => Ok(EscapedCharacter::Zero),
            _ => Err(()),
        }
    }

    pub fn as_raw_string(&self) -> String {
        let mut result = String::new();
        result.push('\\');
        match self {
            EscapedCharacter::Newline => result.push('n'),
            EscapedCharacter::CarriageReturn => result.push('r'),
            EscapedCharacter::Tab => result.push('t'),
            EscapedCharacter::Quote => result.push('"'),
            EscapedCharacter::Zero => result.push('0'),
        }
        result
    }

    pub fn as_string(&self) -> String {
        let mut result = String::new();
        match self {
            EscapedCharacter::Newline => result.push('\n'),
            EscapedCharacter::CarriageReturn => result.push('\r'),
            EscapedCharacter::Tab => result.push('\t'),
            EscapedCharacter::Quote => result.push('\"'),
            EscapedCharacter::Zero => result.push('\0'),
        }
        result
    }

    pub fn as_char(&self) -> char {
        match self {
            EscapedCharacter::Newline => '\n',
            EscapedCharacter::CarriageReturn => '\r',
            EscapedCharacter::Tab => '\t',
            EscapedCharacter::Quote => '"',
            EscapedCharacter::Zero => '\0',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringTokenPart {
    Literal(String),
    // Expression(ASTExpression),
    EscapeSequence(EscapedCharacter),
}
