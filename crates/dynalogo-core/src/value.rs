//! Core Logo values.
//!
//! DynaLOGO starts with the v0.1 value set: words, numbers, and lists.
//! Arrays/property lists arrive later, but the design leaves room for them.
//!
//! UCBLogo treats words case-insensitively for most name/equality purposes
//! while preserving spelling for printing. `Interner` therefore interns both
//! an exact spelling and a canonical ASCII-lowercase spelling.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

impl Symbol {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CanonicalSymbol(u32);

#[derive(Debug, Clone)]
struct SymbolEntry {
    spelling: String,
    canonical: CanonicalSymbol,
}

#[derive(Debug, Default, Clone)]
pub struct Interner {
    exact: HashMap<String, Symbol>,
    canonical: HashMap<String, CanonicalSymbol>,
    canonical_spellings: Vec<String>,
    entries: Vec<SymbolEntry>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a word while preserving its spelling.
    pub fn intern(&mut self, spelling: impl AsRef<str>) -> Symbol {
        let spelling = spelling.as_ref();
        if let Some(symbol) = self.exact.get(spelling) {
            return *symbol;
        }

        let canonical_spelling = canonicalize_word(spelling);
        let canonical = match self.canonical.get(&canonical_spelling) {
            Some(canonical) => *canonical,
            None => {
                let canonical = CanonicalSymbol(self.canonical_spellings.len() as u32);
                self.canonical_spellings.push(canonical_spelling.clone());
                self.canonical.insert(canonical_spelling, canonical);
                canonical
            }
        };

        let symbol = Symbol(self.entries.len() as u32);
        self.entries.push(SymbolEntry {
            spelling: spelling.to_string(),
            canonical,
        });
        self.exact.insert(spelling.to_string(), symbol);
        symbol
    }

    pub fn spelling(&self, symbol: Symbol) -> &str {
        &self.entries[symbol.index()].spelling
    }

    pub fn canonical_spelling(&self, symbol: Symbol) -> &str {
        let canonical = self.entries[symbol.index()].canonical;
        &self.canonical_spellings[canonical.0 as usize]
    }

    pub fn equal_symbols(&self, a: Symbol, b: Symbol) -> bool {
        self.entries[a.index()].canonical == self.entries[b.index()].canonical
    }
}

fn canonicalize_word(word: &str) -> String {
    // UCBLogo is historically ASCII-oriented. Unicode case-folding can be
    // revisited if DynaLOGO deliberately grows non-ASCII Logo word semantics.
    word.to_ascii_lowercase()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogoNumber(f64);

impl LogoNumber {
    pub fn new(value: f64) -> Self {
        Self(value)
    }

    pub fn get(self) -> f64 {
        self.0
    }

    pub fn is_integerish(self) -> bool {
        self.0.is_finite() && self.0.fract() == 0.0
    }
}

impl From<f64> for LogoNumber {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Word(Symbol),
    BareWord(Symbol),
    Number(LogoNumber),
    List(List),
    Array(LogoArray),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self.word_symbol(), other.word_symbol()) {
            (Some(a), Some(b)) => a == b,
            _ => match (self, other) {
                (Value::Number(a), Value::Number(b)) => a == b,
                (Value::List(a), Value::List(b)) => a == b,
                (Value::Array(a), Value::Array(b)) => a.ptr_eq(b),
                _ => false,
            },
        }
    }
}

impl Value {
    pub fn word(interner: &mut Interner, spelling: impl AsRef<str>) -> Self {
        Self::Word(interner.intern(spelling))
    }

    pub fn bare_word(interner: &mut Interner, spelling: impl AsRef<str>) -> Self {
        Self::BareWord(interner.intern(spelling))
    }

    pub fn number(value: f64) -> Self {
        Self::Number(LogoNumber::new(value))
    }

    pub fn list(values: impl IntoIterator<Item = Value>) -> Self {
        Self::List(List::from_values(values))
    }

    pub fn array(size: usize) -> Self {
        Self::Array(LogoArray::new(size))
    }

    pub fn word_symbol(&self) -> Option<Symbol> {
        match self {
            Value::Word(symbol) | Value::BareWord(symbol) => Some(*symbol),
            Value::Number(_) | Value::List(_) | Value::Array(_) => None,
        }
    }

    pub fn as_number(&self, interner: &Interner) -> Option<f64> {
        match self {
            Value::Number(number) => Some(number.get()),
            Value::Word(symbol) | Value::BareWord(symbol) => {
                parse_logo_number(interner.spelling(*symbol))
            }
            Value::List(_) | Value::Array(_) => None,
        }
    }

    pub fn is_empty_list(&self) -> bool {
        matches!(self, Value::List(list) if list.is_empty())
    }

    pub fn equalp(&self, other: &Value, interner: &Interner) -> bool {
        match (self.word_symbol(), other.word_symbol()) {
            (Some(a), Some(b)) => interner.equal_symbols(a, b),
            _ => match (self, other) {
                (Value::Number(a), Value::Number(b)) => number_equal(a.get(), b.get()),
                (Value::List(a), Value::List(b)) => a.equalp(b, interner),
                (Value::Array(a), Value::Array(b)) => a.equalp(b, interner),
                _ => match (self.as_number(interner), other.as_number(interner)) {
                    (Some(a), Some(b)) => number_equal(a, b),
                    _ => false,
                },
            },
        }
    }

    pub fn show(&self, interner: &Interner) -> String {
        match self {
            Value::Word(symbol) | Value::BareWord(symbol) => interner.spelling(*symbol).to_string(),
            Value::Number(number) => format_logo_number(number.get()),
            Value::List(list) => list.show(interner),
            Value::Array(array) => array.show(interner),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogoArray {
    values: Arc<RwLock<Vec<Value>>>,
    origin: isize,
}

impl LogoArray {
    pub fn new(size: usize) -> Self {
        Self {
            values: Arc::new(RwLock::new(vec![Value::List(List::empty()); size])),
            origin: 1,
        }
    }

    pub fn from_values(values: impl IntoIterator<Item = Value>) -> Self {
        Self {
            values: Arc::new(RwLock::new(values.into_iter().collect())),
            origin: 1,
        }
    }

    pub fn len(&self) -> usize {
        self.values.read().expect("array lock poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn origin(&self) -> isize {
        self.origin
    }

    pub fn item(&self, index: isize) -> Option<Value> {
        let zero_based = usize::try_from(index - self.origin).ok()?;
        self.values
            .read()
            .expect("array lock poisoned")
            .get(zero_based)
            .cloned()
    }

    pub fn set_item(&self, index: isize, value: Value) -> bool {
        let Ok(zero_based) = usize::try_from(index - self.origin) else {
            return false;
        };
        let mut values = self.values.write().expect("array lock poisoned");
        let Some(slot) = values.get_mut(zero_based) else {
            return false;
        };
        *slot = value;
        true
    }

    pub fn to_list(&self) -> List {
        List::from_values(self.values.read().expect("array lock poisoned").clone())
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.values, &other.values)
    }

    fn equalp(&self, other: &Self, interner: &Interner) -> bool {
        let a = self.values.read().expect("array lock poisoned");
        let b = other.values.read().expect("array lock poisoned");
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| a.equalp(b, interner))
    }

    fn show(&self, interner: &Interner) -> String {
        let mut out = String::from("{");
        for (i, value) in self
            .values
            .read()
            .expect("array lock poisoned")
            .iter()
            .enumerate()
        {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(&value.show(interner));
        }
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct List(Option<Arc<ConsCell>>);

#[derive(Debug, Clone, PartialEq)]
struct ConsCell {
    head: Value,
    tail: List,
}

impl List {
    pub fn empty() -> Self {
        Self(None)
    }

    pub fn cons(head: Value, tail: List) -> Self {
        Self(Some(Arc::new(ConsCell { head, tail })))
    }

    pub fn from_values(values: impl IntoIterator<Item = Value>) -> Self {
        let values: Vec<Value> = values.into_iter().collect();
        values
            .into_iter()
            .rev()
            .fold(List::empty(), |tail, head| List::cons(head, tail))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn first(&self) -> Option<&Value> {
        self.0.as_deref().map(|cell| &cell.head)
    }

    pub fn butfirst(&self) -> Option<&List> {
        self.0.as_deref().map(|cell| &cell.tail)
    }

    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn item(&self, one_based_index: usize) -> Option<&Value> {
        if one_based_index == 0 {
            return None;
        }
        self.iter().nth(one_based_index - 1)
    }

    pub fn iter(&self) -> ListIter<'_> {
        ListIter {
            next: self.0.as_deref(),
        }
    }

    pub fn pointer_identity(&self) -> Option<usize> {
        self.0.as_ref().map(|cell| Arc::as_ptr(cell) as usize)
    }

    pub fn equalp(&self, other: &List, interner: &Interner) -> bool {
        let mut a = self.iter();
        let mut b = other.iter();
        loop {
            match (a.next(), b.next()) {
                (None, None) => return true,
                (Some(left), Some(right)) if left.equalp(right, interner) => {}
                _ => return false,
            }
        }
    }

    pub fn show(&self, interner: &Interner) -> String {
        let mut out = String::from("[");
        for (i, value) in self.iter().enumerate() {
            if i > 0 {
                out.push(' ');
            }
            out.push_str(&value.show(interner));
        }
        out.push(']');
        out
    }
}

pub struct ListIter<'a> {
    next: Option<&'a ConsCell>,
}

impl<'a> Iterator for ListIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        let cell = self.next?;
        self.next = cell.tail.0.as_deref();
        Some(&cell.head)
    }
}

pub fn parse_logo_number(text: &str) -> Option<f64> {
    if text.is_empty() {
        return None;
    }
    let value: f64 = text.parse().ok()?;
    if value.is_finite() {
        Some(value)
    } else {
        None
    }
}

pub fn format_logo_number(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    if value.is_finite() && value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_992.0 {
        return format!("{value:.0}");
    }
    let formatted = format!("{value}");
    if formatted.contains('.') && !formatted.contains('e') && !formatted.contains('E') {
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    } else {
        formatted
    }
}

fn number_equal(a: f64, b: f64) -> bool {
    a == b
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interner_preserves_spelling_but_compares_canonically() {
        let mut interner = Interner::new();
        let hello = interner.intern("Hello");
        let hello_again = interner.intern("Hello");
        let shout = interner.intern("HELLO");

        assert_eq!(hello, hello_again);
        assert_ne!(hello, shout);
        assert_eq!(interner.spelling(hello), "Hello");
        assert_eq!(interner.spelling(shout), "HELLO");
        assert_eq!(interner.canonical_spelling(hello), "hello");
        assert!(interner.equal_symbols(hello, shout));
    }

    #[test]
    fn logo_number_formatting() {
        assert_eq!(format_logo_number(42.0), "42");
        assert_eq!(format_logo_number(-0.0), "0");
        assert_eq!(format_logo_number(3.5), "3.5");
        assert_eq!(format_logo_number(1.2500), "1.25");
    }

    #[test]
    fn parses_finite_numbers_only() {
        assert_eq!(parse_logo_number("1.5e-3"), Some(0.0015));
        assert_eq!(parse_logo_number("abc"), None);
        assert_eq!(parse_logo_number("inf"), None);
    }

    #[test]
    fn words_and_numbers_compare_numerically_when_possible() {
        let mut interner = Interner::new();
        let word_one = Value::word(&mut interner, "1");
        let number_one = Value::number(1.0);
        let word_hello = Value::word(&mut interner, "HELLO");
        let word_hello2 = Value::word(&mut interner, "hello");

        assert!(word_one.equalp(&number_one, &interner));
        assert!(word_hello.equalp(&word_hello2, &interner));
        assert!(!word_hello.equalp(&number_one, &interner));
    }

    #[test]
    fn lists_support_first_butfirst_len_item_and_identity() {
        let mut interner = Interner::new();
        let list = List::from_values([
            Value::word(&mut interner, "a"),
            Value::number(2.0),
            Value::word(&mut interner, "c"),
        ]);

        assert_eq!(list.len(), 3);
        assert_eq!(list.first().unwrap().show(&interner), "a");
        assert_eq!(list.item(2).unwrap().show(&interner), "2");
        assert!(list.item(0).is_none());
        assert_eq!(list.butfirst().unwrap().len(), 2);
        assert!(list.pointer_identity().is_some());
    }

    #[test]
    fn lists_compare_recursively_with_equalp() {
        let mut interner = Interner::new();
        let a = List::from_values([
            Value::word(&mut interner, "Hello"),
            Value::list([Value::word(&mut interner, "1")]),
        ]);
        let b = List::from_values([
            Value::word(&mut interner, "hello"),
            Value::list([Value::number(1.0)]),
        ]);

        assert!(a.equalp(&b, &interner));
        assert_eq!(Value::List(a).show(&interner), "[Hello [1]]");
    }

    #[test]
    fn empty_lists_show_as_square_brackets() {
        let interner = Interner::new();
        assert!(List::empty().is_empty());
        assert_eq!(List::empty().show(&interner), "[]");
    }
}
