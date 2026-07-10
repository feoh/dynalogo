//! UCBLogo tokenization.
//!
//! Notable UCBLogo rules honored here:
//! - `[ ] ( ) { }` are self-delimiting.
//! - Infix operator characters (`+ - * / = < > <>`) delimit *bare* and
//!   `:dots` words, but are ordinary characters inside a `"quoted` word.
//! - `\` escapes the next character; `|bars|` quote a run of characters
//!   (including whitespace and delimiters) verbatim.
//! - `;` starts a comment running to end of line.
//! - `~` as the last character of a line continues it onto the next line,
//!   including inside comments.
//! - `e`/`E` exponent signs are kept inside numeric literals, so `1.5e-3`
//!   is one word rather than `1.5e` `-` `3`.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    Less,
    Greater,
    LessEq,
    GreaterEq,
    NotEq,
}

impl fmt::Display for InfixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            InfixOp::Plus => "+",
            InfixOp::Minus => "-",
            InfixOp::Star => "*",
            InfixOp::Slash => "/",
            InfixOp::Equal => "=",
            InfixOp::Less => "<",
            InfixOp::Greater => ">",
            InfixOp::LessEq => "<=",
            InfixOp::GreaterEq => ">=",
            InfixOp::NotEq => "<>",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Bare word: procedure name, number literal, etc.
    Word(String),
    /// `"word` — evaluates to the word itself.
    QuotedWord(String),
    /// `:word` — shorthand for `THING "word`.
    ColonWord(String),
    Infix(InfixOp),
    LBracket,
    RBracket,
    LParen,
    RParen,
    LBrace,
    RBrace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    /// 1-based physical line.
    pub line: usize,
    /// 1-based column of the token's first character.
    pub col: usize,
    /// Whitespace (or start of input) immediately precedes this token.
    /// The parser needs this to resolve unary vs. binary minus.
    pub space_before: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (line {}, column {})",
            self.message, self.line, self.col
        )
    }
}

impl std::error::Error for LexError {}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).run()
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    col: usize,
    space_before: bool,
    tokens: Vec<Token>,
}

const OPERATOR_CHARS: &[char] = &['+', '-', '*', '/', '=', '<', '>'];
const GROUPING_CHARS: &[char] = &['[', ']', '(', ')', '{', '}'];

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Lexer {
            chars: source.chars().peekable(),
            line: 1,
            col: 1,
            space_before: true,
            tokens: Vec::new(),
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn error(&self, message: impl Into<String>) -> LexError {
        LexError {
            message: message.into(),
            line: self.line,
            col: self.col,
        }
    }

    /// Consume `~\n` if the next characters are exactly that, returning true.
    fn try_continuation(&mut self) -> bool {
        if self.peek() == Some('~') {
            let mut lookahead = self.chars.clone();
            lookahead.next();
            match lookahead.peek() {
                Some('\n') | None => {
                    self.bump(); // ~
                    self.bump(); // \n (or end of input)
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn push(&mut self, kind: TokenKind, line: usize, col: usize) {
        let space_before = self.space_before;
        self.tokens.push(Token {
            kind,
            line,
            col,
            space_before,
        });
        self.space_before = false;
    }

    fn run(mut self) -> Result<Vec<Token>, LexError> {
        while let Some(c) = self.peek() {
            if self.try_continuation() {
                continue;
            }
            if c.is_whitespace() {
                self.bump();
                self.space_before = true;
                continue;
            }
            if c == ';' {
                self.consume_comment();
                self.space_before = true;
                continue;
            }

            let (line, col) = (self.line, self.col);
            match c {
                '[' => {
                    self.bump();
                    self.push(TokenKind::LBracket, line, col);
                }
                ']' => {
                    self.bump();
                    self.push(TokenKind::RBracket, line, col);
                }
                '(' => {
                    self.bump();
                    self.push(TokenKind::LParen, line, col);
                }
                ')' => {
                    self.bump();
                    self.push(TokenKind::RParen, line, col);
                }
                '{' => {
                    self.bump();
                    self.push(TokenKind::LBrace, line, col);
                }
                '}' => {
                    self.bump();
                    self.push(TokenKind::RBrace, line, col);
                }
                '+' | '-' | '*' | '/' | '=' | '<' | '>' => {
                    let op = self.consume_operator();
                    self.push(TokenKind::Infix(op), line, col);
                }
                '"' => {
                    self.bump();
                    let word = self.consume_word_body(false)?;
                    self.push(TokenKind::QuotedWord(word), line, col);
                }
                ':' => {
                    self.bump();
                    let word = self.consume_word_body(true)?;
                    if word.is_empty() {
                        return Err(self.error("expected a name after `:`"));
                    }
                    self.push(TokenKind::ColonWord(word), line, col);
                }
                _ => {
                    let word = self.consume_word_body(true)?;
                    self.push(TokenKind::Word(word), line, col);
                }
            }
        }
        Ok(self.tokens)
    }

    fn consume_comment(&mut self) {
        loop {
            match self.peek() {
                None => return,
                Some('\n') => {
                    self.bump();
                    return;
                }
                Some('~') => {
                    // `~` at end of line continues the comment.
                    if !self.try_continuation() {
                        self.bump();
                    }
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    fn consume_operator(&mut self) -> InfixOp {
        let c = self.bump().expect("operator start");
        match c {
            '+' => InfixOp::Plus,
            '-' => InfixOp::Minus,
            '*' => InfixOp::Star,
            '/' => InfixOp::Slash,
            '=' => InfixOp::Equal,
            '>' => {
                if self.peek() == Some('=') {
                    self.bump();
                    InfixOp::GreaterEq
                } else {
                    InfixOp::Greater
                }
            }
            '<' => match self.peek() {
                Some('=') => {
                    self.bump();
                    InfixOp::LessEq
                }
                Some('>') => {
                    self.bump();
                    InfixOp::NotEq
                }
                _ => InfixOp::Less,
            },
            _ => unreachable!(),
        }
    }

    /// Read a word body. `operators_delimit` is false for `"quoted` words,
    /// where operator characters are ordinary.
    fn consume_word_body(&mut self, operators_delimit: bool) -> Result<String, LexError> {
        let mut out = String::new();
        loop {
            if self.peek() == Some('~') && self.try_continuation() {
                continue;
            }
            let Some(c) = self.peek() else { break };
            if c.is_whitespace() || GROUPING_CHARS.contains(&c) || c == ';' {
                break;
            }
            if operators_delimit && OPERATOR_CHARS.contains(&c) {
                if self.is_exponent_sign(&out, c) {
                    self.bump();
                    out.push(c);
                    continue;
                }
                break;
            }
            match c {
                '\\' => {
                    self.bump();
                    match self.bump() {
                        Some('\n') | None => {
                            return Err(
                                self.error("`\\` at end of line (use `~` to continue a line)")
                            );
                        }
                        Some(escaped) => out.push(escaped),
                    }
                }
                '|' => {
                    self.bump();
                    self.consume_bars(&mut out)?;
                }
                _ => {
                    self.bump();
                    out.push(c);
                }
            }
        }
        Ok(out)
    }

    /// `1.5e-3` stays one word: a `+`/`-` directly after a trailing `e`/`E`
    /// of a numeric prefix is part of the literal, and a digit must follow.
    fn is_exponent_sign(&mut self, so_far: &str, c: char) -> bool {
        if c != '+' && c != '-' {
            return false;
        }
        let Some(last) = so_far.chars().last() else {
            return false;
        };
        if last != 'e' && last != 'E' {
            return false;
        }
        let mantissa = &so_far[..so_far.len() - 1];
        if mantissa.is_empty()
            || !mantissa.chars().all(|m| m.is_ascii_digit() || m == '.')
            || !mantissa.chars().any(|m| m.is_ascii_digit())
        {
            return false;
        }
        let mut lookahead = self.chars.clone();
        lookahead.next(); // the sign
        matches!(lookahead.peek(), Some(d) if d.is_ascii_digit())
    }

    fn consume_bars(&mut self, out: &mut String) -> Result<(), LexError> {
        loop {
            match self.bump() {
                None => return Err(self.error("unterminated `|`")),
                Some('|') => return Ok(()),
                Some('\\') => match self.bump() {
                    None => return Err(self.error("unterminated `|`")),
                    Some(escaped) => out.push(escaped),
                },
                Some(c) => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source).unwrap().into_iter().map(|t| t.kind).collect()
    }

    fn word(s: &str) -> TokenKind {
        TokenKind::Word(s.to_string())
    }

    #[test]
    fn basic_words() {
        assert_eq!(kinds("print hello"), vec![word("print"), word("hello")]);
    }

    #[test]
    fn brackets_and_parens_self_delimit() {
        assert_eq!(
            kinds("repeat 4[fd 100]"),
            vec![
                word("repeat"),
                word("4"),
                TokenKind::LBracket,
                word("fd"),
                word("100"),
                TokenKind::RBracket,
            ]
        );
        assert_eq!(
            kinds("(sum 1 2 3)"),
            vec![
                TokenKind::LParen,
                word("sum"),
                word("1"),
                word("2"),
                word("3"),
                TokenKind::RParen,
            ]
        );
    }

    #[test]
    fn quoted_and_colon_words() {
        assert_eq!(
            kinds("make \"x :y"),
            vec![
                word("make"),
                TokenKind::QuotedWord("x".to_string()),
                TokenKind::ColonWord("y".to_string()),
            ]
        );
    }

    #[test]
    fn operators_delimit_bare_words() {
        assert_eq!(
            kinds("2+3*:n"),
            vec![
                word("2"),
                TokenKind::Infix(InfixOp::Plus),
                word("3"),
                TokenKind::Infix(InfixOp::Star),
                TokenKind::ColonWord("n".to_string()),
            ]
        );
    }

    #[test]
    fn operators_do_not_delimit_quoted_words() {
        assert_eq!(
            kinds("print \"a+b"),
            vec![word("print"), TokenKind::QuotedWord("a+b".to_string())]
        );
    }

    #[test]
    fn two_char_operators() {
        assert_eq!(
            kinds("1<=2 3>=4 5<>6"),
            vec![
                word("1"),
                TokenKind::Infix(InfixOp::LessEq),
                word("2"),
                word("3"),
                TokenKind::Infix(InfixOp::GreaterEq),
                word("4"),
                word("5"),
                TokenKind::Infix(InfixOp::NotEq),
                word("6"),
            ]
        );
    }

    #[test]
    fn exponent_stays_in_number() {
        assert_eq!(kinds("1.5e-3"), vec![word("1.5e-3")]);
        assert_eq!(kinds("2E+10"), vec![word("2E+10")]);
        // Not an exponent: delimits as usual.
        assert_eq!(
            kinds("apple-3"),
            vec![word("apple"), TokenKind::Infix(InfixOp::Minus), word("3")]
        );
        assert_eq!(
            kinds("1.5e-x"),
            vec![word("1.5e"), TokenKind::Infix(InfixOp::Minus), word("x")]
        );
    }

    #[test]
    fn comments() {
        assert_eq!(
            kinds("fd 100 ; go forward\nrt 90"),
            vec![word("fd"), word("100"), word("rt"), word("90")]
        );
    }

    #[test]
    fn comment_continuation() {
        assert_eq!(
            kinds("; comment goes on~\nstill comment\nfd 1"),
            vec![word("fd"), word("1")]
        );
    }

    #[test]
    fn line_continuation_in_word() {
        assert_eq!(kinds("for~\nward"), vec![word("forward")]);
    }

    #[test]
    fn backslash_escapes() {
        assert_eq!(
            kinds("print \"a\\ b"),
            vec![word("print"), TokenKind::QuotedWord("a b".to_string())]
        );
        assert_eq!(
            kinds("\"a\\[b"),
            vec![TokenKind::QuotedWord("a[b".to_string())]
        );
    }

    #[test]
    fn bars_quote_verbatim() {
        assert_eq!(
            kinds("print \"|hello world|"),
            vec![
                word("print"),
                TokenKind::QuotedWord("hello world".to_string())
            ]
        );
        assert_eq!(kinds("|a b|c"), vec![word("a bc")]);
    }

    #[test]
    fn unterminated_bar_is_an_error() {
        assert!(lex("print \"|oops").is_err());
    }

    #[test]
    fn empty_quoted_word() {
        assert_eq!(
            kinds("print \" "),
            vec![word("print"), TokenKind::QuotedWord(String::new())]
        );
    }

    #[test]
    fn colon_requires_name() {
        assert!(lex("print :").is_err());
    }

    #[test]
    fn space_before_tracking_for_unary_minus() {
        let tokens = lex("fd -5 3-4").unwrap();
        let minus_positions: Vec<(usize, bool)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.kind == TokenKind::Infix(InfixOp::Minus))
            .map(|(i, t)| (i, t.space_before))
            .collect();
        // `-5`: minus has space before; `3-4`: minus does not.
        assert_eq!(minus_positions, vec![(1, true), (4, false)]);
        // And the token after `-5`'s minus has no space before it.
        assert!(!tokens[2].space_before);
    }

    #[test]
    fn positions() {
        let tokens = lex("fd 100\nrt 90").unwrap();
        assert_eq!((tokens[0].line, tokens[0].col), (1, 1));
        assert_eq!((tokens[1].line, tokens[1].col), (1, 4));
        assert_eq!((tokens[2].line, tokens[2].col), (2, 1));
        assert_eq!((tokens[3].line, tokens[3].col), (2, 4));
    }
}
