//! A reader for Valve's text KeyValues format, used by Steam's `.vdf` and
//! `.acf` files.
//!
//! A document is a list of key-value pairs; a value is either a string or a
//! nested block in braces. Keys and strings may be quoted (with `\\`, `\"`,
//! `\n` and `\t` escapes) or bare. `//` starts a comment outside quotes.
//! Keys are looked up regardless of case, as Steam itself does.

use std::fmt;

/// The value of a key: a string or a nested block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Block(Block),
}

/// A list of key-value pairs, in file order. A document is a block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Block {
    entries: Vec<(String, Value)>,
}

impl Block {
    /// The value of the first entry whose key matches, ignoring case.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }

    /// The nested block under `key`, if there is one.
    pub fn block(&self, key: &str) -> Option<&Block> {
        match self.get(key)? {
            Value::Block(block) => Some(block),
            Value::String(_) => None,
        }
    }

    /// The string under `key`, if there is one.
    pub fn string(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Value::String(s) => Some(s),
            Value::Block(_) => None,
        }
    }

    /// All entries in file order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Why a document could not be read, and on which line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    line: usize,
    message: &'static str,
}

impl ParseError {
    /// The line (counted from 1) where the problem was found.
    pub fn line(&self) -> usize {
        self.line
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "malformed KeyValues on line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Reads a whole KeyValues document.
pub fn parse(text: &str) -> Result<Block, ParseError> {
    let mut lexer = Lexer::new(text.strip_prefix('\u{feff}').unwrap_or(text));
    let block = read_block(&mut lexer)?;
    match lexer.next()? {
        None => Ok(block),
        Some(_) => Err(lexer.error("closing brace without an opening one")),
    }
}

/// Reads entries until a closing brace (left for the caller) or the end.
fn read_block(lexer: &mut Lexer) -> Result<Block, ParseError> {
    let mut entries = Vec::new();
    loop {
        let key = match lexer.peek()? {
            None | Some(Token::Close) => return Ok(Block { entries }),
            Some(Token::Open) => return Err(lexer.error("block without a key")),
            Some(Token::Text(_)) => match lexer.next()? {
                Some(Token::Text(key)) => key,
                _ => unreachable!("peeked a text token"),
            },
        };
        let value = match lexer.next()? {
            Some(Token::Text(value)) => Value::String(value),
            Some(Token::Open) => {
                let block = read_block(lexer)?;
                match lexer.next()? {
                    Some(Token::Close) => Value::Block(block),
                    _ => return Err(lexer.error("block is not closed")),
                }
            }
            Some(Token::Close) | None => return Err(lexer.error("key without a value")),
        };
        entries.push((key, value));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Open,
    Close,
    Text(String),
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    peeked: Option<Option<Token>>,
}

impl<'a> Lexer<'a> {
    fn new(text: &'a str) -> Self {
        Self { chars: text.chars().peekable(), line: 1, peeked: None }
    }

    fn error(&self, message: &'static str) -> ParseError {
        ParseError { line: self.line, message }
    }

    fn peek(&mut self) -> Result<Option<&Token>, ParseError> {
        if self.peeked.is_none() {
            self.peeked = Some(self.read()?);
        }
        Ok(self.peeked.as_ref().and_then(Option::as_ref))
    }

    fn next(&mut self) -> Result<Option<Token>, ParseError> {
        match self.peeked.take() {
            Some(token) => Ok(token),
            None => self.read(),
        }
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next();
        if c == Some('\n') {
            self.line += 1;
        }
        c
    }

    fn read(&mut self) -> Result<Option<Token>, ParseError> {
        loop {
            match self.chars.peek() {
                None => return Ok(None),
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') => {
                    self.bump();
                    if self.chars.peek() != Some(&'/') {
                        return Ok(Some(Token::Text(self.bare(String::from("/")))));
                    }
                    while !matches!(self.chars.peek(), None | Some('\n')) {
                        self.bump();
                    }
                }
                Some('{') => {
                    self.bump();
                    return Ok(Some(Token::Open));
                }
                Some('}') => {
                    self.bump();
                    return Ok(Some(Token::Close));
                }
                Some('"') => {
                    self.bump();
                    return self.quoted().map(|s| Some(Token::Text(s)));
                }
                Some(_) => return Ok(Some(Token::Text(self.bare(String::new())))),
            }
        }
    }

    fn quoted(&mut self) -> Result<String, ParseError> {
        let mut text = String::new();
        loop {
            match self.bump() {
                None => return Err(self.error("string is not closed")),
                Some('"') => return Ok(text),
                Some('\\') => match self.bump() {
                    Some('n') => text.push('\n'),
                    Some('t') => text.push('\t'),
                    Some(c @ ('\\' | '"')) => text.push(c),
                    Some(c) => {
                        text.push('\\');
                        text.push(c);
                    }
                    None => return Err(self.error("string is not closed")),
                },
                Some(c) => text.push(c),
            }
        }
    }

    fn bare(&mut self, mut text: String) -> String {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() || matches!(c, '{' | '}' | '"') {
                break;
            }
            if c == '/' && self.chars.clone().nth(1) == Some('/') {
                break; // a comment follows directly
            }
            text.push(c);
            self.bump();
        }
        text
    }
}
