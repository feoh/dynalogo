//! Parser for Logo instruction lists.
//!
//! The lexer only knows tokens. Logo parsing needs procedure arities: `REPEAT`
//! consumes two inputs, `PRINT` consumes one, `(SUM 1 2 3)` consumes all inputs
//! until `)`, and infix operators bind between expressions. This module keeps
//! that arity-driven shape explicit so user procedures can extend the table
//! later when `TO ... END` is implemented.

use std::collections::HashMap;
use std::fmt;

use crate::lexer::{lex, InfixOp, LexError, Token, TokenKind};
use crate::value::{parse_logo_number, Interner, List, Symbol, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arity {
    Exact(usize),
}

#[derive(Debug, Clone)]
pub struct ArityTable {
    arities: HashMap<String, Arity>,
}

impl ArityTable {
    pub fn new() -> Self {
        Self {
            arities: HashMap::new(),
        }
    }

    pub fn with_core_v0_1() -> Self {
        let mut table = Self::new();
        for (name, arity) in [
            ("OUTPUT", 1),
            ("OP", 1),
            ("STOP", 0),
            ("REPEAT", 2),
            ("FOREVER", 1),
            ("IF", 2),
            ("IFELSE", 3),
            ("RUN", 1),
            ("RUNRESULT", 1),
            ("PARSE", 1),
            ("RUNPARSE", 1),
            ("APPLY", 2),
            ("FOREACH", 2),
            ("MAP", 2),
            ("FILTER", 2),
            ("REDUCE", 2),
            ("REPCOUNT", 0),
            ("TEST", 1),
            ("IFTRUE", 1),
            ("IFT", 1),
            ("IFFALSE", 1),
            ("IFF", 1),
            ("WAIT", 1),
            ("CATCH", 2),
            ("THROW", 2),
            ("ERROR", 0),
            ("PAUSE", 0),
            ("CONTINUE", 0),
            ("FIRST", 1),
            ("BUTFIRST", 1),
            ("BF", 1),
            ("LAST", 1),
            ("BUTLAST", 1),
            ("BL", 1),
            ("FPUT", 2),
            ("LPUT", 2),
            ("SENTENCE", 2),
            ("SE", 2),
            ("LIST", 2),
            ("WORD", 2),
            ("COUNT", 1),
            ("ITEM", 2),
            ("WHICH", 2),
            ("EMPTYP", 1),
            ("EMPTY?", 1),
            ("EQUALP", 2),
            ("EQUAL?", 2),
            ("MEMBERP", 2),
            ("MEMBER?", 2),
            ("SUM", 2),
            ("DIFFERENCE", 2),
            ("PRODUCT", 2),
            ("QUOTIENT", 2),
            ("REMAINDER", 2),
            ("ABS", 1),
            ("INT", 1),
            ("ROUND", 1),
            ("SQRT", 1),
            ("SIN", 1),
            ("COS", 1),
            ("TAN", 1),
            ("RANDOM", 1),
            ("RERANDOM", 0),
            ("AND", 2),
            ("OR", 2),
            ("NOT", 1),
            ("PRINT", 1),
            ("PR", 1),
            ("SHOW", 1),
            ("TYPE", 1),
            ("READLIST", 0),
            ("RL", 0),
            ("MAKE", 2),
            ("NAME", 2),
            ("THING", 1),
            ("LOCAL", 1),
            ("NAMEP", 1),
            ("WORDP", 1),
            ("LISTP", 1),
            ("NUMBERP", 1),
            ("INTP", 1),
            ("DECIMALP", 1),
            ("DEFINEDP", 1),
            ("DEFINED?", 1),
            ("PRIMITIVEP", 1),
            ("PRIMITIVE?", 1),
            ("TEXT", 1),
            ("FULLTEXT", 1),
            ("COPYDEF", 2),
            ("DEFINE", 3),
            ("PO", 1),
            ("POALL", 0),
            ("PONS", 0),
            ("POPS", 0),
            ("POTS", 0),
            (".PRIMITIVES", 0),
            ("ERASE", 1),
            ("ER", 1),
            ("ERN", 1),
            ("ERNS", 0),
            ("ERPS", 0),
            ("ERALL", 0),
            ("PPROP", 3),
            ("GPROP", 2),
            ("REMPROP", 2),
            ("PLIST", 1),
            ("ARRAY", 1),
            ("SETITEM", 3),
            ("LISTTOARRAY", 1),
            ("ARRAYTOLIST", 1),
            ("FORWARD", 1),
            ("FD", 1),
            ("BACK", 1),
            ("BK", 1),
            ("LEFT", 1),
            ("LT", 1),
            ("RIGHT", 1),
            ("RT", 1),
            ("SETXY", 2),
            ("SETX", 1),
            ("SETY", 1),
            ("SETPOS", 1),
            ("SETHEADING", 1),
            ("SETH", 1),
            ("HOME", 0),
            ("CLEARSCREEN", 0),
            ("CS", 0),
            ("PENUP", 0),
            ("PU", 0),
            ("PENDOWN", 0),
            ("PD", 0),
            ("SETPENCOLOR", 1),
            ("SETPC", 1),
            ("SETPENSIZE", 1),
            ("HIDETURTLE", 0),
            ("HT", 0),
            ("INIT.TURTLE", 0),
            ("SHOWTURTLE", 0),
            ("ST", 0),
            ("SHOWNP", 0),
            ("POS", 0),
            ("HEADING", 0),
            ("XCOR", 0),
            ("YCOR", 0),
        ] {
            table.insert(name, Arity::Exact(arity));
        }
        table
    }

    pub fn insert(&mut self, name: impl AsRef<str>, arity: Arity) {
        self.arities
            .insert(name.as_ref().to_ascii_lowercase(), arity);
    }

    pub fn get(&self, name: &str) -> Option<Arity> {
        self.arities.get(&name.to_ascii_lowercase()).copied()
    }

    pub fn remove(&mut self, name: &str) {
        self.arities.remove(&name.to_ascii_lowercase());
    }
}

impl Default for ArityTable {
    fn default() -> Self {
        Self::with_core_v0_1()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    expressions: Vec<Expr>,
}

impl Program {
    pub fn expressions(&self) -> &[Expr] {
        &self.expressions
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    /// `:name` shorthand for `THING "name`.
    Thing(Symbol),
    Call {
        callee: Symbol,
        args: Vec<Expr>,
        greedy: bool,
    },
    Infix {
        op: InfixOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (line {}, column {})",
            self.message, self.line, self.col
        )
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(value: LexError) -> Self {
        ParseError {
            message: value.message,
            line: value.line,
            col: value.col,
        }
    }
}

pub fn parse_source(
    source: &str,
    interner: &mut Interner,
    arities: &ArityTable,
) -> Result<Program, ParseError> {
    let tokens = lex(source)?;
    Parser::new(tokens, interner, arities).parse_program()
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    interner: &'a mut Interner,
    arities: &'a ArityTable,
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token>, interner: &'a mut Interner, arities: &'a ArityTable) -> Self {
        Self {
            tokens,
            pos: 0,
            interner,
            arities,
        }
    }

    fn parse_program(mut self) -> Result<Program, ParseError> {
        let mut expressions = Vec::new();
        while !self.is_at_end() {
            if self.check_closer() {
                let token = self.peek().expect("checked not end");
                return Err(self.error_at(token, "unexpected closing delimiter"));
            }
            expressions.push(self.parse_expression(0)?);
        }
        Ok(Program { expressions })
    }

    fn parse_expression(&mut self, min_precedence: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        while let Some(op) = self.peek_infix() {
            let precedence = infix_precedence(op);
            if precedence < min_precedence {
                break;
            }
            self.advance();
            let right = self.parse_expression(precedence + 1)?;
            left = Expr::Infix {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let token = match self.advance() {
            Some(token) => token.clone(),
            None => return Err(self.error_at_end("expected an expression")),
        };
        match token.kind {
            TokenKind::Word(word) => self.word_expr(word, token.line, token.col),
            TokenKind::QuotedWord(word) => Ok(Expr::Literal(Value::word(self.interner, word))),
            TokenKind::ColonWord(word) => Ok(Expr::Thing(self.interner.intern(word))),
            TokenKind::LBracket => self.list_literal(token.line, token.col).map(Expr::Literal),
            TokenKind::LParen => self.greedy_call(token.line, token.col),
            TokenKind::Infix(InfixOp::Minus) => self.unary_minus(token.line, token.col),
            TokenKind::Infix(op) => {
                Err(self.error_at(&token, format!("unexpected infix operator `{op}`")))
            }
            TokenKind::RBracket | TokenKind::RParen | TokenKind::RBrace | TokenKind::LBrace => {
                Err(self.error_at(&token, "unexpected delimiter"))
            }
        }
    }

    fn word_expr(&mut self, word: String, line: usize, col: usize) -> Result<Expr, ParseError> {
        if let Some(number) = parse_logo_number(&word) {
            return Ok(Expr::Literal(Value::number(number)));
        }

        let callee = self.interner.intern(&word);
        match self.arities.get(&word) {
            Some(Arity::Exact(arity)) => {
                let mut args = Vec::with_capacity(arity);
                for _ in 0..arity {
                    match self.parse_expression(0) {
                        Ok(arg) => args.push(arg),
                        Err(error)
                            if error.line == 0
                                || error.message == "unexpected delimiter"
                                || error.message == "unexpected closing delimiter" =>
                        {
                            return Err(ParseError {
                                message: format!("not enough inputs to {word}"),
                                line,
                                col,
                            });
                        }
                        Err(error) => return Err(error),
                    }
                }
                Ok(Expr::Call {
                    callee,
                    args,
                    greedy: false,
                })
            }
            None => Ok(Expr::Call {
                callee,
                args: Vec::new(),
                greedy: false,
            }),
        }
    }

    fn greedy_call(&mut self, line: usize, col: usize) -> Result<Expr, ParseError> {
        let Some(callee_token) = self.advance().cloned() else {
            return Err(ParseError {
                message: "expected a procedure name after `(`".to_string(),
                line,
                col,
            });
        };
        let TokenKind::Word(word) = callee_token.kind else {
            return Err(self.error_at(&callee_token, "expected a procedure name after `(`"));
        };
        let callee = self.interner.intern(word);
        let mut args = Vec::new();
        while !self.is_at_end() && !matches!(self.peek_kind(), Some(TokenKind::RParen)) {
            args.push(self.parse_expression(0)?);
        }
        self.expect(
            TokenKind::RParen,
            "expected `)` to close parenthesized call",
        )?;
        Ok(Expr::Call {
            callee,
            args,
            greedy: true,
        })
    }

    fn unary_minus(&mut self, line: usize, col: usize) -> Result<Expr, ParseError> {
        if let Some(next) = self.peek() {
            if !next.space_before {
                if let TokenKind::Word(word) = &next.kind {
                    if let Some(number) = parse_logo_number(word) {
                        self.advance();
                        return Ok(Expr::Literal(Value::number(-number)));
                    }
                }
            }
        }

        let callee = self.interner.intern("-");
        let arg = self.parse_expression(unary_precedence())?;
        if self.is_at_end() || line > 0 || col > 0 {
            Ok(Expr::Call {
                callee,
                args: vec![arg],
                greedy: false,
            })
        } else {
            unreachable!()
        }
    }

    fn list_literal(&mut self, line: usize, col: usize) -> Result<Value, ParseError> {
        let mut values = Vec::new();
        while !self.is_at_end() && !matches!(self.peek_kind(), Some(TokenKind::RBracket)) {
            values.push(self.literal_value()?);
        }
        self.expect(TokenKind::RBracket, "expected `]` to close list literal")
            .map_err(|mut error| {
                if error.line == 0 {
                    error.line = line;
                    error.col = col;
                }
                error
            })?;
        Ok(Value::List(List::from_values(values)))
    }

    fn literal_value(&mut self) -> Result<Value, ParseError> {
        let token = match self.advance() {
            Some(token) => token.clone(),
            None => return Err(self.error_at_end("expected list item")),
        };
        match token.kind {
            TokenKind::Word(word) => Ok(number_or_word(self.interner, word)),
            TokenKind::QuotedWord(word) => Ok(Value::word(self.interner, word)),
            TokenKind::ColonWord(word) => Ok(Value::word(self.interner, format!(":{word}"))),
            TokenKind::Infix(op) => Ok(Value::word(self.interner, op.to_string())),
            TokenKind::LBracket => self.list_literal(token.line, token.col),
            TokenKind::LParen => Ok(Value::word(self.interner, "(")),
            TokenKind::RParen => Ok(Value::word(self.interner, ")")),
            TokenKind::LBrace => Ok(Value::word(self.interner, "{")),
            TokenKind::RBrace => Ok(Value::word(self.interner, "}")),
            TokenKind::RBracket => Err(self.error_at(&token, "unexpected `]`")),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|token| token.kind.clone())
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.pos);
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn check_closer(&self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::RBracket | TokenKind::RParen | TokenKind::RBrace)
        )
    }

    fn peek_infix(&self) -> Option<InfixOp> {
        match self.peek_kind() {
            Some(TokenKind::Infix(op)) => Some(op),
            _ => None,
        }
    }

    fn expect(&mut self, expected: TokenKind, message: &str) -> Result<(), ParseError> {
        let Some(token) = self.advance().cloned() else {
            return Err(self.error_at_end(message));
        };
        if token.kind == expected {
            Ok(())
        } else {
            Err(self.error_at(&token, message))
        }
    }

    fn error_at(&self, token: &Token, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            line: token.line,
            col: token.col,
        }
    }

    fn error_at_end(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            line: 0,
            col: 0,
        }
    }
}

fn number_or_word(interner: &mut Interner, word: String) -> Value {
    match parse_logo_number(&word) {
        Some(number) => Value::number(number),
        None => Value::word(interner, word),
    }
}

fn unary_precedence() -> u8 {
    5
}

fn infix_precedence(op: InfixOp) -> u8 {
    match op {
        InfixOp::Star | InfixOp::Slash => 4,
        InfixOp::Plus | InfixOp::Minus => 3,
        InfixOp::Equal
        | InfixOp::Less
        | InfixOp::Greater
        | InfixOp::LessEq
        | InfixOp::GreaterEq
        | InfixOp::NotEq => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> (Program, Interner) {
        let mut interner = Interner::new();
        let program = parse_source(source, &mut interner, &ArityTable::default()).unwrap();
        (program, interner)
    }

    fn sym_name(interner: &Interner, symbol: Symbol) -> &str {
        interner.spelling(symbol)
    }

    #[test]
    fn parses_core_calls_by_arity() {
        let (program, interner) = parse("print sum 1 2");
        assert_eq!(program.expressions().len(), 1);
        let Expr::Call {
            callee,
            args,
            greedy,
        } = &program.expressions()[0]
        else {
            panic!("expected PRINT call");
        };
        assert_eq!(sym_name(&interner, *callee), "print");
        assert!(!greedy);
        assert_eq!(args.len(), 1);
        let Expr::Call { callee, args, .. } = &args[0] else {
            panic!("expected SUM call");
        };
        assert_eq!(sym_name(&interner, *callee), "sum");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn parses_repeat_with_instruction_list_literal() {
        let (program, interner) = parse("repeat 4 [fd 100]");
        let Expr::Call { callee, args, .. } = &program.expressions()[0] else {
            panic!("expected REPEAT");
        };
        assert_eq!(sym_name(&interner, *callee), "repeat");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], Expr::Literal(Value::number(4.0)));
        assert_eq!(
            args[1].clone(),
            Expr::Literal(Value::list([
                Value::word(&mut interner.clone(), "fd"),
                Value::number(100.0)
            ]))
        );
    }

    #[test]
    fn parses_infix_with_precedence() {
        let (program, _) = parse("2+3*4");
        let Expr::Infix { op, left, right } = &program.expressions()[0] else {
            panic!("expected infix");
        };
        assert_eq!(*op, InfixOp::Plus);
        assert_eq!(**left, Expr::Literal(Value::number(2.0)));
        let Expr::Infix { op, left, right } = &**right else {
            panic!("expected product on right");
        };
        assert_eq!(*op, InfixOp::Star);
        assert_eq!(**left, Expr::Literal(Value::number(3.0)));
        assert_eq!(**right, Expr::Literal(Value::number(4.0)));
    }

    #[test]
    fn parses_parenthesized_greedy_call() {
        let (program, interner) = parse("print (sum 1 2 3)");
        let Expr::Call { args, .. } = &program.expressions()[0] else {
            panic!("expected PRINT");
        };
        let Expr::Call {
            callee,
            args,
            greedy,
        } = &args[0]
        else {
            panic!("expected greedy SUM");
        };
        assert_eq!(sym_name(&interner, *callee), "sum");
        assert!(*greedy);
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn parses_quoted_words_and_dots() {
        let (program, interner) = parse("make \"x :y");
        let Expr::Call { callee, args, .. } = &program.expressions()[0] else {
            panic!("expected MAKE");
        };
        assert_eq!(sym_name(&interner, *callee), "make");
        assert_eq!(args.len(), 2);
        let Expr::Literal(Value::Word(name)) = args[0] else {
            panic!("expected quoted word");
        };
        assert_eq!(sym_name(&interner, name), "x");
        let Expr::Thing(thing) = args[1] else {
            panic!("expected :y");
        };
        assert_eq!(sym_name(&interner, thing), "y");
    }

    #[test]
    fn parses_negative_number_as_literal() {
        let (program, _) = parse("print -5");
        let Expr::Call { args, .. } = &program.expressions()[0] else {
            panic!("expected PRINT");
        };
        assert_eq!(args[0], Expr::Literal(Value::number(-5.0)));
    }

    #[test]
    fn list_literals_are_data_not_calls() {
        let (program, interner) = parse("[print sum 1 2 [fd 100]]");
        let Expr::Literal(Value::List(list)) = &program.expressions()[0] else {
            panic!("expected list");
        };
        assert_eq!(list.show(&interner), "[print sum 1 2 [fd 100]]");
    }

    #[test]
    fn reports_unclosed_list() {
        let mut interner = Interner::new();
        let error = parse_source("[fd 100", &mut interner, &ArityTable::default()).unwrap_err();
        assert!(error.message.contains("]"));
    }

    #[test]
    fn reports_not_enough_inputs_to_known_procedure() {
        let mut interner = Interner::new();
        let error = parse_source("print", &mut interner, &ArityTable::default()).unwrap_err();
        assert_eq!(error.message, "not enough inputs to print");
    }
}
