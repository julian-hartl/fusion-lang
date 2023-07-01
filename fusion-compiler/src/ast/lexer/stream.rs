use crate::ast::lexer::Lexer;
use crate::ast::lexer::token::{Token, TokenKind};

const MAX_LOOKAHEAD: usize = 10;

pub struct TokenStream<'a> {
    buffer: [Option<Token>; MAX_LOOKAHEAD],
    lexer: &'a mut Lexer<'a>,
}

impl<'a> TokenStream<'a> {
    pub fn new(lexer: &'a mut Lexer<'a>) -> Self {
        Self {
            buffer: [None, None, None, None, None, None, None, None, None, None],
            lexer,
        }
    }

    pub fn prepare(&mut self) {
        self.fill_buffer();
    }

    pub fn peek(&self, mut offset: usize) -> &Token {
        if self.buffer[1].is_none() {
            offset = 0;
        }
        self.buffer[offset].as_ref().unwrap_or_else(|| panic!("BUG: We read past the end of the stream: {:?}", self.buffer))
    }

    pub fn consume(&mut self) -> Option<Token> {
        let token = self.buffer.first_mut().unwrap().take();
        self.shift_buffer();
        token
    }

    pub fn consume_if(&mut self, kind: TokenKind) -> Option<Token> {
        if self.peek(0).kind == kind {
            return self.consume();
        }
        None
    }

    pub fn consume_or_eof(&mut self) -> Token {
        self.consume().unwrap_or_else(|| self.current().clone())
    }

    pub fn is_at_end(&self) -> bool {
        self.peek(0).kind == TokenKind::Eof
    }

    pub fn current(&self) -> &Token {
        self.peek(0)
    }

    fn shift_buffer(&mut self) {
        while let Some(token) = self.lexer.next_token() {
            match &token.kind {
                TokenKind::Whitespace => continue,
                TokenKind::Newline => continue,
                _ => {
                    for i in 0..MAX_LOOKAHEAD - 1 {
                        self.buffer[i] = self.buffer[i + 1].take();
                    }
                    self.buffer[MAX_LOOKAHEAD - 1] = Some(token);
                    break;
                }
            }
        };
        assert!(self.buffer[0].is_some());
    }

    fn fill_buffer(&mut self) {
        for slot in &mut self.buffer {
            assert!(slot.is_none());
            while let Some(token) = self.lexer.next_token() {
                match &token.kind {
                    TokenKind::Whitespace => continue,
                    TokenKind::Newline => continue,
                    _ => {
                        *slot = Some(token);
                        break;
                    }
                }
            }
        }
    }
}

impl Drop for TokenStream<'_> {
    fn drop(&mut self) {
        assert!(self.is_at_end());
    }
}
