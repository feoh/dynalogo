//! Stack VM for DynaLOGO bytecode.
//!
//! This is the initial v0.1 executor. It establishes the pieces that later
//! procedure support will build on: a shared interner, dynamic-scope frame
//! stack, primitive dispatch, bytecode stack execution, and `OUTPUT`/`STOP`
//! control signals.

use std::collections::HashMap;
use std::fmt;

use crate::bytecode::{Chunk, Compiler, Instruction};
use crate::lexer::{lex, InfixOp, TokenKind};
use crate::parser::{parse_source, Arity, ArityTable};
use crate::value::{Interner, List, LogoNumber, Symbol, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlow {
    None,
    Output(Value),
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    pub stack: Vec<Value>,
    pub output: String,
    pub control: ControlFlow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmError {
    pub message: String,
}

impl VmError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for VmError {}

#[derive(Debug, Default, Clone)]
pub struct Environment {
    globals: HashMap<String, Value>,
    frames: Vec<HashMap<String, Value>>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_frame(&mut self) {
        self.frames.push(HashMap::new());
    }

    pub fn pop_frame(&mut self) {
        self.frames.pop();
    }

    pub fn define_local(&mut self, name: impl Into<String>, value: Value) {
        let key = name.into().to_ascii_lowercase();
        match self.frames.last_mut() {
            Some(frame) => {
                frame.insert(key, value);
            }
            None => {
                self.globals.insert(key, value);
            }
        }
    }

    pub fn set_global(&mut self, name: impl Into<String>, value: Value) {
        self.globals.insert(name.into().to_ascii_lowercase(), value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        let key = name.to_ascii_lowercase();
        self.frames
            .iter()
            .rev()
            .find_map(|frame| frame.get(&key))
            .or_else(|| self.globals.get(&key))
    }
}

#[derive(Debug, Clone)]
pub struct Vm {
    interner: Interner,
    env: Environment,
    output: String,
    arities: ArityTable,
    procedures: HashMap<String, Procedure>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Procedure {
    name: Symbol,
    params: Vec<Symbol>,
    chunk: Chunk,
}

impl Procedure {
    pub fn name(&self) -> Symbol {
        self.name
    }

    pub fn params(&self) -> &[Symbol] {
        &self.params
    }

    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self {
            interner: Interner::new(),
            env: Environment::new(),
            output: String::new(),
            arities: ArityTable::default(),
            procedures: HashMap::new(),
        }
    }
}

impl Vm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn interner(&self) -> &Interner {
        &self.interner
    }

    pub fn interner_mut(&mut self) -> &mut Interner {
        &mut self.interner
    }

    pub fn env(&self) -> &Environment {
        &self.env
    }

    pub fn env_mut(&mut self) -> &mut Environment {
        &mut self.env
    }

    pub fn output(&self) -> &str {
        &self.output
    }

    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    pub fn procedures(&self) -> &HashMap<String, Procedure> {
        &self.procedures
    }

    pub fn eval_source(&mut self, source: &str) -> Result<RunResult, VmError> {
        let runnable = self.define_procedures_in_source(source)?;
        if runnable.trim().is_empty() {
            return Ok(RunResult {
                stack: Vec::new(),
                output: self.output.clone(),
                control: ControlFlow::None,
            });
        }
        let program = parse_source(&runnable, &mut self.interner, &self.arities)
            .map_err(|error| VmError::new(error.to_string()))?;
        let chunk = Compiler::new()
            .compile_program(&program)
            .map_err(|error| VmError::new(error.to_string()))?;
        self.run(&chunk)
    }

    pub fn define_procedure(
        &mut self,
        name: impl AsRef<str>,
        params: Vec<String>,
        body: &str,
    ) -> Result<(), VmError> {
        let name = name.as_ref();
        let name_symbol = self.interner.intern(name);
        let param_symbols: Vec<Symbol> = params
            .iter()
            .map(|param| self.interner.intern(param))
            .collect();
        self.arities.insert(name, Arity::Exact(param_symbols.len()));
        let program = parse_source(body, &mut self.interner, &self.arities)
            .map_err(|error| VmError::new(error.to_string()))?;
        let chunk = Compiler::new()
            .compile_program(&program)
            .map_err(|error| VmError::new(error.to_string()))?;
        self.procedures.insert(
            name.to_ascii_lowercase(),
            Procedure {
                name: name_symbol,
                params: param_symbols,
                chunk,
            },
        );
        Ok(())
    }

    pub fn run(&mut self, chunk: &Chunk) -> Result<RunResult, VmError> {
        let mut stack = Vec::new();
        let mut ip = 0;
        let control = loop {
            let instruction = chunk
                .instructions()
                .get(ip)
                .ok_or_else(|| VmError::new("instruction pointer ran past end of chunk"))?;
            ip += 1;

            match instruction {
                Instruction::Push(value) => stack.push(value.clone()),
                Instruction::LoadThing(symbol) => {
                    let value = self.load_thing(*symbol)?;
                    stack.push(value);
                }
                Instruction::Call { callee, argc } => {
                    let args = pop_args(&mut stack, *argc)?;
                    match self.call(*callee, args)? {
                        PrimitiveResult::Value(value) => stack.push(value),
                        PrimitiveResult::NoValue => {}
                        PrimitiveResult::Control(control) => break control,
                    }
                }
                Instruction::Infix(op) => {
                    let right = stack
                        .pop()
                        .ok_or_else(|| VmError::new("infix operator missing right input"))?;
                    let left = stack
                        .pop()
                        .ok_or_else(|| VmError::new("infix operator missing left input"))?;
                    stack.push(self.eval_infix(*op, left, right)?);
                }
                Instruction::Halt => break ControlFlow::None,
            }
        };

        Ok(RunResult {
            stack,
            output: self.output.clone(),
            control,
        })
    }

    fn load_thing(&self, symbol: Symbol) -> Result<Value, VmError> {
        let name = self.interner.spelling(symbol);
        self.env
            .get(name)
            .cloned()
            .ok_or_else(|| VmError::new(format!("{name} has no value")))
    }

    fn define_procedures_in_source(&mut self, source: &str) -> Result<String, VmError> {
        let mut runnable = Vec::new();
        let mut lines = source.lines().peekable();

        while let Some(line) = lines.next() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !starts_with_logo_word(trimmed, "to") {
                runnable.push(line.to_string());
                continue;
            }

            let (name, params) = parse_to_header(trimmed)?;
            let mut body = Vec::new();
            let mut saw_end = false;
            for body_line in lines.by_ref() {
                if body_line.trim().eq_ignore_ascii_case("end") {
                    saw_end = true;
                    break;
                }
                body.push(body_line.to_string());
            }
            if !saw_end {
                return Err(VmError::new(format!("procedure {name} is missing END")));
            }
            self.define_procedure(name, params, &body.join("\n"))?;
        }

        Ok(runnable.join("\n"))
    }

    fn call(&mut self, callee: Symbol, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        let name = self.interner.canonical_spelling(callee).to_string();
        match name.as_str() {
            "sum" | "+" => self.number_binop(args, |a, b| a + b),
            "difference" | "-" => self.number_binop(args, |a, b| a - b),
            "product" | "*" => self.number_binop(args, |a, b| a * b),
            "quotient" | "/" => self.number_binop(args, |a, b| a / b),
            "remainder" => self.number_binop(args, |a, b| a % b),
            "and" => self.logic_and(args),
            "or" => self.logic_or(args),
            "not" => self.logic_not(args),
            "equalp" | "equal?" => self.equalp(args),
            "emptyp" | "empty?" => self.emptyp(args),
            "memberp" | "member?" => self.memberp(args),
            "first" => self.first(args),
            "butfirst" | "bf" => self.butfirst(args),
            "last" => self.last(args),
            "butlast" | "bl" => self.butlast(args),
            "fput" => self.fput(args),
            "lput" => self.lput(args),
            "sentence" | "se" => self.sentence(args),
            "list" => self.list(args),
            "word" => self.word(args),
            "count" => self.count(args),
            "item" => self.item(args),
            "print" | "pr" => self.print(args),
            "show" => self.show(args),
            "type" => self.r#type(args),
            "readlist" | "rl" => self.readlist(args),
            "make" | "name" => self.make(args),
            "thing" => self.thing(args),
            "local" => self.local(args),
            "repeat" => self.repeat(args),
            "if" => self.r#if(args),
            "ifelse" => self.ifelse(args),
            "run" => self.run_list(args),
            "repcount" => self.repcount(args),
            "output" | "op" => self.output_control(args),
            "stop" => {
                expect_arity(&name, &args, 0).map(|()| PrimitiveResult::Control(ControlFlow::Stop))
            }
            _ => self.call_user_procedure(&name, args),
        }
    }

    fn call_user_procedure(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<PrimitiveResult, VmError> {
        let procedure = self
            .procedures
            .get(name)
            .cloned()
            .ok_or_else(|| VmError::new(format!("I don't know how to {name}")))?;
        expect_arity(name, &args, procedure.params.len())?;

        self.env.push_frame();
        for (param, value) in procedure.params.iter().zip(args) {
            let name = self.interner.spelling(*param).to_string();
            self.env.define_local(name, value);
        }
        let result = self.run(&procedure.chunk);
        self.env.pop_frame();

        match result?.control {
            ControlFlow::None | ControlFlow::Stop => Ok(PrimitiveResult::NoValue),
            ControlFlow::Output(value) => Ok(PrimitiveResult::Value(value)),
        }
    }

    fn eval_infix(&mut self, op: InfixOp, left: Value, right: Value) -> Result<Value, VmError> {
        let args = vec![left, right];
        match op {
            InfixOp::Plus => self.number_binop(args, |a, b| a + b),
            InfixOp::Minus => self.number_binop(args, |a, b| a - b),
            InfixOp::Star => self.number_binop(args, |a, b| a * b),
            InfixOp::Slash => self.number_binop(args, |a, b| a / b),
            InfixOp::Equal => self.equalp(args),
            InfixOp::Less => self.number_compare(args, |a, b| a < b),
            InfixOp::Greater => self.number_compare(args, |a, b| a > b),
            InfixOp::LessEq => self.number_compare(args, |a, b| a <= b),
            InfixOp::GreaterEq => self.number_compare(args, |a, b| a >= b),
            InfixOp::NotEq => self.not_equalp(args),
        }
        .map(|result| match result {
            PrimitiveResult::Value(value) => value,
            PrimitiveResult::NoValue | PrimitiveResult::Control(_) => {
                unreachable!("infix ops always output")
            }
        })
    }

    fn number_binop(
        &mut self,
        args: Vec<Value>,
        op: impl FnOnce(f64, f64) -> f64,
    ) -> Result<PrimitiveResult, VmError> {
        expect_arity("number operation", &args, 2)?;
        let a = number_input(&args[0], &self.interner)?;
        let b = number_input(&args[1], &self.interner)?;
        Ok(PrimitiveResult::Value(Value::Number(LogoNumber::new(op(
            a, b,
        )))))
    }

    fn number_compare(
        &mut self,
        args: Vec<Value>,
        op: impl FnOnce(f64, f64) -> bool,
    ) -> Result<PrimitiveResult, VmError> {
        expect_arity("number comparison", &args, 2)?;
        let a = number_input(&args[0], &self.interner)?;
        let b = number_input(&args[1], &self.interner)?;
        Ok(PrimitiveResult::Value(self.logo_bool(op(a, b))))
    }

    fn logic_and(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("and", &args, 2)?;
        Ok(PrimitiveResult::Value(self.logo_bool(
            logo_truth(&args[0], &self.interner) && logo_truth(&args[1], &self.interner),
        )))
    }

    fn logic_or(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("or", &args, 2)?;
        Ok(PrimitiveResult::Value(self.logo_bool(
            logo_truth(&args[0], &self.interner) || logo_truth(&args[1], &self.interner),
        )))
    }

    fn logic_not(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("not", &args, 1)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(!logo_truth(&args[0], &self.interner)),
        ))
    }

    fn equalp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("equalp", &args, 2)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(args[0].equalp(&args[1], &self.interner)),
        ))
    }

    fn not_equalp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("<>", &args, 2)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(!args[0].equalp(&args[1], &self.interner)),
        ))
    }

    fn emptyp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("emptyp", &args, 1)?;
        let empty = match &args[0] {
            Value::Word(symbol) => self.interner.spelling(*symbol).is_empty(),
            Value::Number(_) => false,
            Value::List(list) => list.is_empty(),
        };
        Ok(PrimitiveResult::Value(self.logo_bool(empty)))
    }

    fn memberp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("memberp", &args, 2)?;
        let member = match &args[1] {
            Value::List(list) => list
                .iter()
                .any(|value| args[0].equalp(value, &self.interner)),
            Value::Word(symbol) => self
                .interner
                .spelling(*symbol)
                .contains(&args[0].show(&self.interner)),
            Value::Number(_) => false,
        };
        Ok(PrimitiveResult::Value(self.logo_bool(member)))
    }

    fn first(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("first", &args, 1)?;
        let value = match &args[0] {
            Value::Word(symbol) => {
                let text = self.interner.spelling(*symbol).to_string();
                first_char_value(&mut self.interner, &text)?
            }
            Value::Number(number) => {
                let text = Value::Number(*number).show(&self.interner);
                first_char_value(&mut self.interner, &text)?
            }
            Value::List(list) => list
                .first()
                .cloned()
                .ok_or_else(|| VmError::new("FIRST of empty list"))?,
        };
        Ok(PrimitiveResult::Value(value))
    }

    fn butfirst(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("butfirst", &args, 1)?;
        let value = match &args[0] {
            Value::Word(symbol) => {
                let text = drop_first_char(self.interner.spelling(*symbol));
                Value::word(&mut self.interner, text)
            }
            Value::Number(number) => {
                let text = drop_first_char(&Value::Number(*number).show(&self.interner));
                Value::word(&mut self.interner, text)
            }
            Value::List(list) => Value::List(list.butfirst().cloned().unwrap_or_else(List::empty)),
        };
        Ok(PrimitiveResult::Value(value))
    }

    fn last(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("last", &args, 1)?;
        let value = match &args[0] {
            Value::Word(symbol) => {
                let text = self.interner.spelling(*symbol).to_string();
                last_char_value(&mut self.interner, &text)?
            }
            Value::Number(number) => {
                let text = Value::Number(*number).show(&self.interner);
                last_char_value(&mut self.interner, &text)?
            }
            Value::List(list) => list
                .iter()
                .last()
                .cloned()
                .ok_or_else(|| VmError::new("LAST of empty list"))?,
        };
        Ok(PrimitiveResult::Value(value))
    }

    fn butlast(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("butlast", &args, 1)?;
        let value = match &args[0] {
            Value::Word(symbol) => {
                let text = drop_last_char(self.interner.spelling(*symbol));
                Value::word(&mut self.interner, text)
            }
            Value::Number(number) => {
                let text = drop_last_char(&Value::Number(*number).show(&self.interner));
                Value::word(&mut self.interner, text)
            }
            Value::List(list) => {
                let mut values: Vec<Value> = list.iter().cloned().collect();
                values.pop();
                Value::List(List::from_values(values))
            }
        };
        Ok(PrimitiveResult::Value(value))
    }

    fn fput(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("fput", &args, 2)?;
        let Value::List(list) = &args[1] else {
            return Err(VmError::new("FPUT second input must be a list"));
        };
        Ok(PrimitiveResult::Value(Value::List(List::cons(
            args[0].clone(),
            list.clone(),
        ))))
    }

    fn lput(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("lput", &args, 2)?;
        let Value::List(list) = &args[1] else {
            return Err(VmError::new("LPUT second input must be a list"));
        };
        let mut values: Vec<Value> = list.iter().cloned().collect();
        values.push(args[0].clone());
        Ok(PrimitiveResult::Value(Value::List(List::from_values(
            values,
        ))))
    }

    fn sentence(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("sentence", &args, 2)?;
        let mut values = Vec::new();
        sentence_part(&args[0], &mut values);
        sentence_part(&args[1], &mut values);
        Ok(PrimitiveResult::Value(Value::List(List::from_values(
            values,
        ))))
    }

    fn list(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("list", &args, 2)?;
        Ok(PrimitiveResult::Value(Value::List(List::from_values(args))))
    }

    fn word(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("word", &args, 2)?;
        let text = format!(
            "{}{}",
            args[0].show(&self.interner),
            args[1].show(&self.interner)
        );
        Ok(PrimitiveResult::Value(Value::word(
            &mut self.interner,
            text,
        )))
    }

    fn count(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("count", &args, 1)?;
        let count = match &args[0] {
            Value::Word(symbol) => self.interner.spelling(*symbol).chars().count(),
            Value::Number(number) => Value::Number(*number).show(&self.interner).chars().count(),
            Value::List(list) => list.len(),
        };
        Ok(PrimitiveResult::Value(Value::number(count as f64)))
    }

    fn item(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("item", &args, 2)?;
        let index = number_input(&args[0], &self.interner)? as usize;
        let value = match &args[1] {
            Value::Word(symbol) => {
                let text = self.interner.spelling(*symbol).to_string();
                nth_char_value(&mut self.interner, &text, index)?
            }
            Value::Number(number) => {
                let text = Value::Number(*number).show(&self.interner);
                nth_char_value(&mut self.interner, &text, index)?
            }
            Value::List(list) => list
                .item(index)
                .cloned()
                .ok_or_else(|| VmError::new("ITEM index out of range"))?,
        };
        Ok(PrimitiveResult::Value(value))
    }

    fn print(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("print", &args, 1)?;
        self.output.push_str(&args[0].show(&self.interner));
        self.output.push('\n');
        Ok(PrimitiveResult::NoValue)
    }

    fn show(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("show", &args, 1)?;
        self.output.push_str(&args[0].show(&self.interner));
        self.output.push('\n');
        Ok(PrimitiveResult::NoValue)
    }

    fn r#type(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("type", &args, 1)?;
        self.output.push_str(&args[0].show(&self.interner));
        Ok(PrimitiveResult::NoValue)
    }

    fn readlist(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("readlist", &args, 0)?;
        Err(VmError::new(
            "READLIST is not connected to an input stream yet",
        ))
    }

    fn make(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("make", &args, 2)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        self.env.set_global(name, args[1].clone());
        Ok(PrimitiveResult::NoValue)
    }

    fn thing(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("thing", &args, 1)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        let value = self
            .env
            .get(&name)
            .cloned()
            .ok_or_else(|| VmError::new(format!("{name} has no value")))?;
        Ok(PrimitiveResult::Value(value))
    }

    fn local(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("local", &args, 1)?;
        let names = local_names(&args[0], &self.interner)?;
        for name in names {
            self.env.define_local(name, Value::List(List::empty()));
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn repeat(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("repeat", &args, 2)?;
        let count = number_input(&args[0], &self.interner)? as usize;
        let list = list_input(&args[1], "REPEAT")?;
        for i in 1..=count {
            self.env.define_local("repcount", Value::number(i as f64));
            match self.execute_instruction_list(list)? {
                ControlFlow::None => {}
                control => return Ok(PrimitiveResult::Control(control)),
            }
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn r#if(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("if", &args, 2)?;
        if logo_truth(&args[0], &self.interner) {
            let list = list_input(&args[1], "IF")?;
            match self.execute_instruction_list(list)? {
                ControlFlow::None => Ok(PrimitiveResult::NoValue),
                control => Ok(PrimitiveResult::Control(control)),
            }
        } else {
            Ok(PrimitiveResult::NoValue)
        }
    }

    fn ifelse(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("ifelse", &args, 3)?;
        let list = if logo_truth(&args[0], &self.interner) {
            list_input(&args[1], "IFELSE")?
        } else {
            list_input(&args[2], "IFELSE")?
        };
        match self.execute_instruction_list(list)? {
            ControlFlow::None => Ok(PrimitiveResult::NoValue),
            control => Ok(PrimitiveResult::Control(control)),
        }
    }

    fn run_list(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("run", &args, 1)?;
        let list = list_input(&args[0], "RUN")?;
        match self.execute_instruction_list(list)? {
            ControlFlow::None => Ok(PrimitiveResult::NoValue),
            control => Ok(PrimitiveResult::Control(control)),
        }
    }

    fn execute_instruction_list(&mut self, list: &List) -> Result<ControlFlow, VmError> {
        let source = list_to_source(list, &self.interner, &self.arities);
        let program = parse_source(&source, &mut self.interner, &self.arities)
            .map_err(|error| VmError::new(error.to_string()))?;
        let chunk = Compiler::new()
            .compile_program(&program)
            .map_err(|error| VmError::new(error.to_string()))?;
        Ok(self.run(&chunk)?.control)
    }

    fn repcount(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("repcount", &args, 0)?;
        let value = self
            .env
            .get("repcount")
            .cloned()
            .ok_or_else(|| VmError::new("REPCOUNT is only available inside REPEAT"))?;
        Ok(PrimitiveResult::Value(value))
    }

    fn output_control(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("output", &args, 1)?;
        Ok(PrimitiveResult::Control(ControlFlow::Output(
            args[0].clone(),
        )))
    }

    fn logo_bool(&mut self, value: bool) -> Value {
        Value::word(&mut self.interner, if value { "true" } else { "false" })
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PrimitiveResult {
    Value(Value),
    NoValue,
    Control(ControlFlow),
}

fn pop_args(stack: &mut Vec<Value>, argc: usize) -> Result<Vec<Value>, VmError> {
    if stack.len() < argc {
        return Err(VmError::new(format!("not enough inputs: expected {argc}")));
    }
    let split_at = stack.len() - argc;
    Ok(stack.split_off(split_at))
}

fn expect_arity(name: &str, args: &[Value], expected: usize) -> Result<(), VmError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(VmError::new(format!(
            "{name} expected {expected} input(s), got {}",
            args.len()
        )))
    }
}

fn number_input(value: &Value, interner: &Interner) -> Result<f64, VmError> {
    value
        .as_number(interner)
        .ok_or_else(|| VmError::new(format!("{} is not a number", value.show(interner))))
}

fn variable_name_input(value: &Value, interner: &Interner) -> Result<String, VmError> {
    match value {
        Value::Word(symbol) => Ok(interner.spelling(*symbol).to_string()),
        _ => Err(VmError::new(format!(
            "{} is not a variable name",
            value.show(interner)
        ))),
    }
}

fn starts_with_logo_word(line: &str, word: &str) -> bool {
    let mut parts = line.split_whitespace();
    matches!(parts.next(), Some(first) if first.eq_ignore_ascii_case(word))
}

fn parse_to_header(line: &str) -> Result<(String, Vec<String>), VmError> {
    let tokens = lex(line).map_err(|error| VmError::new(error.to_string()))?;
    let mut iter = tokens.into_iter();
    match iter.next().map(|token| token.kind) {
        Some(TokenKind::Word(word)) if word.eq_ignore_ascii_case("to") => {}
        _ => return Err(VmError::new("expected TO header")),
    }

    let name = match iter.next().map(|token| token.kind) {
        Some(TokenKind::Word(name)) => name,
        _ => return Err(VmError::new("TO requires a procedure name")),
    };

    let mut params = Vec::new();
    for token in iter {
        match token.kind {
            TokenKind::ColonWord(param) => params.push(param),
            _ => return Err(VmError::new("TO parameters must be written as :name")),
        }
    }
    Ok((name, params))
}

fn list_input<'a>(value: &'a Value, name: &str) -> Result<&'a List, VmError> {
    match value {
        Value::List(list) => Ok(list),
        _ => Err(VmError::new(format!("{name} input must be a list"))),
    }
}

fn sentence_part(value: &Value, values: &mut Vec<Value>) {
    match value {
        Value::List(list) => values.extend(list.iter().cloned()),
        _ => values.push(value.clone()),
    }
}

fn local_names(value: &Value, interner: &Interner) -> Result<Vec<String>, VmError> {
    match value {
        Value::Word(symbol) => Ok(vec![interner.spelling(*symbol).to_string()]),
        Value::List(list) => list
            .iter()
            .map(|value| variable_name_input(value, interner))
            .collect(),
        _ => Err(VmError::new(format!(
            "{} is not a variable name or list of names",
            value.show(interner)
        ))),
    }
}

fn logo_truth(value: &Value, interner: &Interner) -> bool {
    match value {
        Value::Word(symbol) => !interner.canonical_spelling(*symbol).eq("false"),
        Value::Number(number) => number.get() != 0.0,
        Value::List(list) => !list.is_empty(),
    }
}

fn first_char_value(interner: &mut Interner, text: &str) -> Result<Value, VmError> {
    let first = text
        .chars()
        .next()
        .ok_or_else(|| VmError::new("FIRST of empty word"))?;
    Ok(Value::word(interner, first.to_string()))
}

fn last_char_value(interner: &mut Interner, text: &str) -> Result<Value, VmError> {
    let last = text
        .chars()
        .next_back()
        .ok_or_else(|| VmError::new("LAST of empty word"))?;
    Ok(Value::word(interner, last.to_string()))
}

fn nth_char_value(
    interner: &mut Interner,
    text: &str,
    one_based_index: usize,
) -> Result<Value, VmError> {
    if one_based_index == 0 {
        return Err(VmError::new("ITEM index out of range"));
    }
    let ch = text
        .chars()
        .nth(one_based_index - 1)
        .ok_or_else(|| VmError::new("ITEM index out of range"))?;
    Ok(Value::word(interner, ch.to_string()))
}

fn drop_first_char(text: &str) -> String {
    text.chars().skip(1).collect()
}

fn drop_last_char(text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    chars.pop();
    chars.into_iter().collect()
}

fn list_to_source(list: &List, interner: &Interner, arities: &ArityTable) -> String {
    list.iter()
        .map(|value| match value {
            Value::List(inner) => format!("[{}]", list_to_source(inner, interner, arities)),
            Value::Word(symbol) => {
                let spelling = interner.spelling(*symbol);
                if arities.get(spelling).is_some() || is_operator_word(spelling) {
                    spelling.to_string()
                } else {
                    format!("\"{spelling}")
                }
            }
            _ => value.show(interner),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_operator_word(text: &str) -> bool {
    matches!(
        text,
        "+" | "-" | "*" | "/" | "=" | "<" | ">" | "<=" | ">=" | "<>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(source: &str) -> Result<(RunResult, Vm), VmError> {
        let mut vm = Vm::new();
        let result = vm.eval_source(source)?;
        Ok((result, vm))
    }

    #[test]
    fn runs_arithmetic_and_print() {
        let (result, _) = run("print sum 1 2").unwrap();
        assert_eq!(result.output, "3\n");
        assert!(result.stack.is_empty());
        assert_eq!(result.control, ControlFlow::None);
    }

    #[test]
    fn runs_infix_precedence() {
        let (result, _) = run("print 2+3*4").unwrap();
        assert_eq!(result.output, "14\n");
    }

    #[test]
    fn make_and_colon_lookup_use_dynamic_environment() {
        let (result, vm) = run("make \"x 42 print :x").unwrap();
        assert_eq!(result.output, "42\n");
        assert_eq!(vm.env().get("x").unwrap().show(vm.interner()), "42");
    }

    #[test]
    fn thing_primitive_reads_variable_by_word_name() {
        let (result, _) = run("make \"x 7 print thing \"x").unwrap();
        assert_eq!(result.output, "7\n");
    }

    #[test]
    fn comparisons_output_true_false_words() {
        let (result, _) = run("print 1<2 print equalp \"HELLO \"hello print 3<>3").unwrap();
        assert_eq!(result.output, "true\ntrue\nfalse\n");
    }

    #[test]
    fn output_stops_chunk_with_control_value() {
        let (result, _) = run("output sum 2 3 print 99").unwrap();
        assert_eq!(result.control, ControlFlow::Output(Value::number(5.0)));
        assert_eq!(result.output, "");
    }

    #[test]
    fn stop_stops_chunk() {
        let (result, _) = run("stop print 99").unwrap();
        assert_eq!(result.control, ControlFlow::Stop);
        assert_eq!(result.output, "");
    }

    #[test]
    fn missing_variable_is_error() {
        let error = run("print :missing").unwrap_err();
        assert!(error.message.contains("missing has no value"));
    }

    #[test]
    fn dynamic_scope_frames_shadow_globals() {
        let mut vm = Vm::new();
        vm.env_mut().set_global("x", Value::number(1.0));
        vm.env_mut().push_frame();
        vm.env_mut().define_local("x", Value::number(2.0));
        assert_eq!(vm.env().get("x"), Some(&Value::number(2.0)));
        vm.env_mut().pop_frame();
        assert_eq!(vm.env().get("x"), Some(&Value::number(1.0)));
    }

    #[test]
    fn word_and_list_primitives() {
        let (result, _) = run("print first \"hello \
             print butfirst \"hello \
             print last \"hello \
             print butlast \"hello \
             print word \"hi \"there \
             print list \"a \"b \
             print sentence [a b] [c d] \
             print fput \"z [a b] \
             print lput \"z [a b]")
        .unwrap();
        assert_eq!(
            result.output,
            "h\nello\no\nhell\nhithere\n[a b]\n[a b c d]\n[z a b]\n[a b z]\n"
        );
    }

    #[test]
    fn count_item_empty_and_member_primitives() {
        let (result, _) = run("print count [a b c] \
             print item 2 [a b c] \
             print emptyp [] \
             print emptyp \" \
             print memberp \"b [a b c] \
             print memberp \"x [a b c]")
        .unwrap();
        assert_eq!(result.output, "3\nb\ntrue\ntrue\ntrue\nfalse\n");
    }

    #[test]
    fn logic_primitives() {
        let (result, _) =
            run("print and \"true \"true print or \"false \"true print not \"false").unwrap();
        assert_eq!(result.output, "true\ntrue\ntrue\n");
    }

    #[test]
    fn repeat_if_ifelse_and_run_execute_instruction_lists() {
        let (result, _) = run("repeat 3 [print repcount] \
             if \"true [print \"yes] \
             ifelse \"false [print \"bad] [print \"good] \
             run [print sum 4 5]")
        .unwrap();
        assert_eq!(result.output, "1\n2\n3\nyes\ngood\n9\n");
    }

    #[test]
    fn defines_and_calls_outputting_procedure() {
        let (result, vm) = run("to square :x\n\
             output product :x :x\n\
             end\n\
             print square 5")
        .unwrap();
        assert_eq!(result.output, "25\n");
        assert!(vm.procedures().contains_key("square"));
    }

    #[test]
    fn procedure_arguments_are_dynamically_scoped() {
        let (result, _) = run("to showx :x\n\
             print :x\n\
             end\n\
             make \"x 10\n\
             showx 99\n\
             print :x")
        .unwrap();
        assert_eq!(result.output, "99\n10\n");
    }

    #[test]
    fn recursive_procedure_can_call_itself() {
        let (result, _) = run("to countdown :n\n\
             if equalp :n 0 [stop]\n\
             print :n\n\
             countdown difference :n 1\n\
             end\n\
             countdown 3")
        .unwrap();
        assert_eq!(result.output, "3\n2\n1\n");
    }
}
