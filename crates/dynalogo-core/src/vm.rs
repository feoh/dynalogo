//! Stack VM for DynaLOGO bytecode.
//!
//! This is the initial v0.1 executor. It establishes the pieces that later
//! procedure support will build on: a shared interner, dynamic-scope frame
//! stack, primitive dispatch, bytecode stack execution, and `OUTPUT`/`STOP`
//! control signals.

use std::collections::HashMap;
use std::fmt;
use std::thread;
use std::time::Duration;

use crate::bytecode::{Chunk, Compiler, Instruction};
use crate::lexer::{lex, InfixOp, TokenKind};
use crate::parser::{parse_source, Arity, ArityTable};
use crate::turtle::{HeadlessTurtleBackend, Point, TurtleWorld};
use crate::value::{Interner, List, LogoArray, LogoNumber, Symbol, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlow {
    None,
    Output(Value),
    Stop,
    Throw { tag: Value, value: Value },
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

const CONTROL_LIBRARY_SOURCE: &str = r#"
TO __WHILELOOP :TEST :BODY
  IF RUNRESULT :TEST [RUN :BODY __WHILELOOP :TEST :BODY]
END

TO WHILE :TEST :BODY
  __WHILELOOP :TEST :BODY
END

TO __UNTILLOOP :TEST :BODY
  IFELSE RUNRESULT :TEST [STOP] [RUN :BODY __UNTILLOOP :TEST :BODY]
END

TO UNTIL :TEST :BODY
  __UNTILLOOP :TEST :BODY
END

TO DO.WHILE :BODY :TEST
  RUN :BODY
  IF RUNRESULT :TEST [DO.WHILE :BODY :TEST]
END

TO __CONDREST :CLAUSES
  IF EMPTYP :CLAUSES [STOP]
  IF RUNRESULT FIRST FIRST :CLAUSES [RUN LAST FIRST :CLAUSES STOP]
  __CONDREST BUTFIRST :CLAUSES
END

TO COND :CLAUSES
  __CONDREST :CLAUSES
END

TO __CASEREST :VALUE :CLAUSES
  IF EMPTYP :CLAUSES [STOP]
  IF EQUALP FIRST FIRST :CLAUSES "ELSE [RUN LAST FIRST :CLAUSES STOP]
  IF MEMBERP :VALUE FIRST FIRST :CLAUSES [RUN LAST FIRST :CLAUSES STOP]
  __CASEREST :VALUE BUTFIRST :CLAUSES
END

TO CASE :VALUE :CLAUSES
  __CASEREST :VALUE :CLAUSES
END

TO __FORLOOP :VAR :CURRENT :LIMIT :STEP :BODY
  IFELSE :STEP >= 0 [
    IF :CURRENT > :LIMIT [STOP]
  ] [
    IF :CURRENT < :LIMIT [STOP]
  ]
  MAKE :VAR :CURRENT
  RUN :BODY
  __FORLOOP :VAR SUM :CURRENT :STEP :LIMIT :STEP :BODY
END

TO FOR :CONTROL :BODY
  IFELSE EQUALP COUNT :CONTROL 4 [
    __FORLOOP FIRST :CONTROL ITEM 2 :CONTROL ITEM 3 :CONTROL ITEM 4 :CONTROL :BODY
  ] [
    IFELSE ITEM 2 :CONTROL <= ITEM 3 :CONTROL [
      __FORLOOP FIRST :CONTROL ITEM 2 :CONTROL ITEM 3 :CONTROL 1 :BODY
    ] [
      __FORLOOP FIRST :CONTROL ITEM 2 :CONTROL ITEM 3 :CONTROL -1 :BODY
    ]
  ]
END
"#;

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

    pub fn set(&mut self, name: impl Into<String>, value: Value) {
        let key = name.into().to_ascii_lowercase();
        if let Some(frame) = self
            .frames
            .iter_mut()
            .rev()
            .find(|frame| frame.contains_key(&key))
        {
            frame.insert(key, value);
        } else {
            self.globals.insert(key, value);
        }
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
    property_lists: HashMap<String, HashMap<String, Value>>,
    turtle: TurtleWorld<HeadlessTurtleBackend>,
    test_result: Option<bool>,
    last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Procedure {
    name: Symbol,
    params: Vec<Symbol>,
    chunk: Chunk,
    body_source: String,
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

    pub fn body_source(&self) -> &str {
        &self.body_source
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
            property_lists: HashMap::new(),
            turtle: TurtleWorld::new(HeadlessTurtleBackend::new()),
            test_result: None,
            last_error: None,
        }
    }
}

impl Vm {
    pub fn new() -> Self {
        let mut vm = Self::default();
        vm.install_control_library();
        vm
    }

    fn install_control_library(&mut self) {
        if let Err(error) = self.eval_source(CONTROL_LIBRARY_SOURCE) {
            panic!("control-library procedures should compile: {}", error);
        }
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

    fn remember_error(&mut self, error: VmError) -> VmError {
        self.last_error = Some(error.message.clone());
        error
    }

    pub fn procedures(&self) -> &HashMap<String, Procedure> {
        &self.procedures
    }

    pub fn property_lists(&self) -> &HashMap<String, HashMap<String, Value>> {
        &self.property_lists
    }

    pub fn turtle(&self) -> &TurtleWorld<HeadlessTurtleBackend> {
        &self.turtle
    }

    pub fn turtle_mut(&mut self) -> &mut TurtleWorld<HeadlessTurtleBackend> {
        &mut self.turtle
    }

    pub fn eval_source(&mut self, source: &str) -> Result<RunResult, VmError> {
        let result = (|| {
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
                .compile_effect_program(&program)
                .map_err(|error| VmError::new(error.to_string()))?;
            self.run(&chunk)
        })();
        result.map_err(|error| self.remember_error(error))
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
            .compile_effect_program(&program)
            .map_err(|error| VmError::new(error.to_string()))?;
        self.procedures.insert(
            name.to_ascii_lowercase(),
            Procedure {
                name: name_symbol,
                params: param_symbols,
                chunk,
                body_source: body.to_string(),
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
                Instruction::Call {
                    callee,
                    argc,
                    expects_value,
                } => {
                    let args = pop_args(&mut stack, *argc)?;
                    match self.call(*callee, args)? {
                        PrimitiveResult::Value(value) => stack.push(value),
                        PrimitiveResult::NoValue => {
                            if *expects_value {
                                return Err(VmError::new(format!(
                                    "{} didn't output a value",
                                    self.interner.spelling(*callee)
                                )));
                            }
                        }
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
                Instruction::CheckNoValue => {
                    if let Some(value) = stack.pop() {
                        return Err(VmError::new(format!(
                            "You don't say what to do with {}",
                            value.show(&self.interner)
                        )));
                    }
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
            "definedp" | "defined?" => self.definedp(args),
            "primitivep" | "primitive?" => self.primitivep(args),
            "text" => self.text(args),
            "fulltext" => self.fulltext(args),
            "pprop" => self.pprop(args),
            "gprop" => self.gprop(args),
            "remprop" => self.remprop(args),
            "plist" => self.plist(args),
            "array" => self.array(args),
            "setitem" => self.setitem(args),
            "listtoarray" => self.listtoarray(args),
            "arraytolist" => self.arraytolist(args),
            "repeat" => self.repeat(args),
            "if" => self.r#if(args),
            "ifelse" => self.ifelse(args),
            "run" => self.run_list(args),
            "runresult" => self.runresult(args),
            "parse" => self.parse(args),
            "runparse" => self.runparse(args),
            "apply" => self.apply(args),
            "foreach" => self.foreach(args),
            "map" => self.map(args),
            "filter" => self.filter(args),
            "reduce" => self.reduce(args),
            "repcount" => self.repcount(args),
            "test" => self.test(args),
            "iftrue" | "ift" => self.iftrue(args),
            "iffalse" | "iff" => self.iffalse(args),
            "wait" => self.wait(args),
            "catch" => self.catch(args),
            "throw" => self.throw(args),
            "error" => self.error(args),
            "pause" => self.pause(args),
            "continue" => self.continue_(args),
            "forward" | "fd" => self.turtle_forward(args),
            "back" | "bk" => self.turtle_back(args),
            "left" | "lt" => self.turtle_left(args),
            "right" | "rt" => self.turtle_right(args),
            "setxy" => self.turtle_setxy(args),
            "setpos" => self.turtle_setpos(args),
            "setheading" | "seth" => self.turtle_setheading(args),
            "home" => self.turtle_home(args),
            "clearscreen" | "cs" => self.turtle_clearscreen(args),
            "penup" | "pu" => self.turtle_penup(args),
            "pendown" | "pd" => self.turtle_pendown(args),
            "setpencolor" | "setpc" => self.turtle_setpencolor(args),
            "setpensize" => self.turtle_setpensize(args),
            "hideturtle" | "ht" => self.turtle_hide(args),
            "showturtle" | "st" => self.turtle_show(args),
            "pos" => self.turtle_pos(args),
            "heading" => self.turtle_heading(args),
            "xcor" => self.turtle_xcor(args),
            "ycor" => self.turtle_ycor(args),
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
            ControlFlow::Throw { tag, value } => {
                Ok(PrimitiveResult::Control(ControlFlow::Throw { tag, value }))
            }
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
            Value::Array(array) => array.is_empty(),
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
            Value::Array(array) => array
                .to_list()
                .iter()
                .any(|value| args[0].equalp(value, &self.interner)),
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
            Value::Array(array) => array
                .item(array.origin())
                .ok_or_else(|| VmError::new("FIRST of empty array"))?,
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
            Value::Array(array) => Value::List(
                array
                    .to_list()
                    .butfirst()
                    .cloned()
                    .unwrap_or_else(List::empty),
            ),
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
            Value::Array(array) => array
                .to_list()
                .iter()
                .last()
                .cloned()
                .ok_or_else(|| VmError::new("LAST of empty array"))?,
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
            Value::Array(array) => {
                let mut values: Vec<Value> = array.to_list().iter().cloned().collect();
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
            Value::Array(array) => array.len(),
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
            Value::Array(array) => array
                .item(index as isize)
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
        self.env.set(name, args[1].clone());
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

    fn definedp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("definedp", &args, 1)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        Ok(PrimitiveResult::Value(self.logo_bool(
            self.procedures.contains_key(&name.to_ascii_lowercase()),
        )))
    }

    fn primitivep(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("primitivep", &args, 1)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(is_primitive_name(&name)),
        ))
    }

    fn text(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("text", &args, 1)?;
        let procedure = self.workspace_procedure(&args[0])?.clone();
        Ok(PrimitiveResult::Value(Value::List(procedure_text(
            &procedure,
            &mut self.interner,
            false,
        )?)))
    }

    fn fulltext(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("fulltext", &args, 1)?;
        let procedure = self.workspace_procedure(&args[0])?.clone();
        Ok(PrimitiveResult::Value(Value::List(procedure_text(
            &procedure,
            &mut self.interner,
            true,
        )?)))
    }

    fn workspace_procedure(&self, value: &Value) -> Result<&Procedure, VmError> {
        let name = variable_name_input(value, &self.interner)?;
        self.procedures
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| VmError::new(format!("I don't know how to {name}")))
    }

    fn pprop(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pprop", &args, 3)?;
        let plist_name = property_key_input(&args[0], &self.interner)?;
        let property_name = property_key_input(&args[1], &self.interner)?;
        self.property_lists
            .entry(plist_name)
            .or_default()
            .insert(property_name, args[2].clone());
        Ok(PrimitiveResult::NoValue)
    }

    fn gprop(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("gprop", &args, 2)?;
        let plist_name = property_key_input(&args[0], &self.interner)?;
        let property_name = property_key_input(&args[1], &self.interner)?;
        let value = self
            .property_lists
            .get(&plist_name)
            .and_then(|plist| plist.get(&property_name))
            .cloned()
            .unwrap_or_else(|| Value::List(List::empty()));
        Ok(PrimitiveResult::Value(value))
    }

    fn remprop(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("remprop", &args, 2)?;
        let plist_name = property_key_input(&args[0], &self.interner)?;
        let property_name = property_key_input(&args[1], &self.interner)?;
        if let Some(plist) = self.property_lists.get_mut(&plist_name) {
            plist.remove(&property_name);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn plist(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("plist", &args, 1)?;
        let plist_name = property_key_input(&args[0], &self.interner)?;
        let mut values = Vec::new();
        if let Some(plist) = self.property_lists.get(&plist_name) {
            let mut entries: Vec<_> = plist.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (name, value) in entries {
                values.push(Value::word(&mut self.interner, name));
                values.push(value.clone());
            }
        }
        Ok(PrimitiveResult::Value(Value::List(List::from_values(
            values,
        ))))
    }

    fn array(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("array", &args, 1)?;
        let size = number_input(&args[0], &self.interner)?;
        if size < 0.0 {
            return Err(VmError::new("ARRAY size must be non-negative"));
        }
        Ok(PrimitiveResult::Value(Value::Array(LogoArray::new(
            size as usize,
        ))))
    }

    fn setitem(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setitem", &args, 3)?;
        let index = number_input(&args[0], &self.interner)? as isize;
        let Value::Array(array) = &args[1] else {
            return Err(VmError::new("SETITEM second input must be an array"));
        };
        if !array.set_item(index, args[2].clone()) {
            return Err(VmError::new("SETITEM index out of range"));
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn listtoarray(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("listtoarray", &args, 1)?;
        let list = list_input(&args[0], "LISTTOARRAY")?;
        Ok(PrimitiveResult::Value(Value::Array(
            LogoArray::from_values(list.iter().cloned()),
        )))
    }

    fn arraytolist(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("arraytolist", &args, 1)?;
        let Value::Array(array) = &args[0] else {
            return Err(VmError::new("ARRAYTOLIST input must be an array"));
        };
        Ok(PrimitiveResult::Value(Value::List(array.to_list())))
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
        Ok(self.execute_instruction_list_effect(list)?.control)
    }

    fn execute_instruction_list_effect(&mut self, list: &List) -> Result<RunResult, VmError> {
        let source = list_to_source(list, &self.interner, &self.arities);
        let program = parse_source(&source, &mut self.interner, &self.arities)
            .map_err(|error| VmError::new(error.to_string()))?;
        let chunk = Compiler::new()
            .compile_effect_program(&program)
            .map_err(|error| VmError::new(error.to_string()))?;
        self.run(&chunk)
    }

    fn execute_instruction_list_result(&mut self, list: &List) -> Result<RunResult, VmError> {
        let source = list_to_source(list, &self.interner, &self.arities);
        let program = parse_source(&source, &mut self.interner, &self.arities)
            .map_err(|error| VmError::new(error.to_string()))?;
        let chunk = Compiler::new()
            .compile_program(&program)
            .map_err(|error| VmError::new(error.to_string()))?;
        self.run(&chunk)
    }

    fn runresult(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("runresult", &args, 1)?;
        let list = list_input(&args[0], "RUNRESULT")?;
        let result = self.execute_instruction_list_result(list)?;
        result_value(result).map(PrimitiveResult::Value)
    }

    fn parse(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("parse", &args, 1)?;
        let text = source_text_input(&args[0], &self.interner);
        let tokens = lex(&text).map_err(|error| VmError::new(error.to_string()))?;
        let values = tokens
            .into_iter()
            .filter_map(|token| token_to_data_value(token.kind, &mut self.interner))
            .collect::<Vec<_>>();
        Ok(PrimitiveResult::Value(Value::List(List::from_values(
            values,
        ))))
    }

    fn runparse(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("runparse", &args, 1)?;
        let text = source_text_input(&args[0], &self.interner);
        let program = parse_source(&text, &mut self.interner, &self.arities)
            .map_err(|error| VmError::new(error.to_string()))?;
        let chunk = Compiler::new()
            .compile_program(&program)
            .map_err(|error| VmError::new(error.to_string()))?;
        result_value(self.run(&chunk)?).map(PrimitiveResult::Value)
    }

    fn apply(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("apply", &args, 2)?;
        let arg_list = list_input(&args[1], "APPLY")?;
        match &args[0] {
            Value::Word(symbol) => {
                let source = format!(
                    "{} {}",
                    self.interner.spelling(*symbol),
                    list_to_source(arg_list, &self.interner, &self.arities)
                );
                let program = parse_source(&source, &mut self.interner, &self.arities)
                    .map_err(|error| VmError::new(error.to_string()))?;
                let chunk = Compiler::new()
                    .compile_program(&program)
                    .map_err(|error| VmError::new(error.to_string()))?;
                result_value(self.run(&chunk)?).map(PrimitiveResult::Value)
            }
            Value::List(template) => self
                .eval_template_with_values(template, list_values(arg_list))
                .map(PrimitiveResult::Value),
            _ => Err(VmError::new(
                "APPLY first input must be a word or template list",
            )),
        }
    }

    fn foreach(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("foreach", &args, 2)?;
        let values = list_values(list_input(&args[1], "FOREACH")?);
        let template = list_input(&args[0], "FOREACH")?.clone();
        for value in values {
            self.execute_template_for_effect(&template, vec![value])?;
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn map(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("map", &args, 2)?;
        let values = list_values(list_input(&args[1], "MAP")?);
        let template = list_input(&args[0], "MAP")?.clone();
        let mut mapped = Vec::new();
        for value in values {
            mapped.push(self.eval_template_with_values(&template, vec![value])?);
        }
        Ok(PrimitiveResult::Value(Value::List(List::from_values(
            mapped,
        ))))
    }

    fn filter(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("filter", &args, 2)?;
        let values = list_values(list_input(&args[1], "FILTER")?);
        let template = list_input(&args[0], "FILTER")?.clone();
        let mut kept = Vec::new();
        for value in values {
            let keep = self.eval_template_with_values(&template, vec![value.clone()])?;
            if logo_truth(&keep, &self.interner) {
                kept.push(value);
            }
        }
        Ok(PrimitiveResult::Value(Value::List(List::from_values(kept))))
    }

    fn reduce(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("reduce", &args, 2)?;
        let mut values = list_values(list_input(&args[1], "REDUCE")?).into_iter();
        let Some(mut acc) = values.next() else {
            return Err(VmError::new("REDUCE cannot reduce an empty list"));
        };
        let template = list_input(&args[0], "REDUCE")?.clone();
        for value in values {
            acc = self.eval_template_with_values(&template, vec![acc, value])?;
        }
        Ok(PrimitiveResult::Value(acc))
    }

    fn eval_template_with_values(
        &mut self,
        template: &List,
        values: Vec<Value>,
    ) -> Result<Value, VmError> {
        let result = self.execute_template_result(template, values)?;
        result_value(result)
    }

    fn execute_template_for_effect(
        &mut self,
        template: &List,
        values: Vec<Value>,
    ) -> Result<ControlFlow, VmError> {
        Ok(self.execute_template_effect(template, values)?.control)
    }

    fn execute_template_effect(
        &mut self,
        template: &List,
        values: Vec<Value>,
    ) -> Result<RunResult, VmError> {
        self.env.push_frame();
        for (index, value) in values.iter().cloned().enumerate() {
            let numbered = format!("?{}", index + 1);
            self.env.define_local(numbered, value.clone());
            if index == 0 {
                self.env.define_local("?", value);
            }
        }
        let result = self.execute_instruction_list_effect(template);
        self.env.pop_frame();
        result
    }

    fn execute_template_result(
        &mut self,
        template: &List,
        values: Vec<Value>,
    ) -> Result<RunResult, VmError> {
        self.env.push_frame();
        for (index, value) in values.iter().cloned().enumerate() {
            let numbered = format!("?{}", index + 1);
            self.env.define_local(numbered, value.clone());
            if index == 0 {
                self.env.define_local("?", value);
            }
        }
        let result = self.execute_instruction_list_result(template);
        self.env.pop_frame();
        result
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

    fn test(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("test", &args, 1)?;
        self.test_result = Some(logo_truth(&args[0], &self.interner));
        Ok(PrimitiveResult::NoValue)
    }

    fn iftrue(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("iftrue", &args, 1)?;
        match self.test_result {
            Some(true) => {
                let list = list_input(&args[0], "IFTRUE")?;
                match self.execute_instruction_list(list)? {
                    ControlFlow::None => Ok(PrimitiveResult::NoValue),
                    control => Ok(PrimitiveResult::Control(control)),
                }
            }
            Some(false) => Ok(PrimitiveResult::NoValue),
            None => Err(VmError::new("TEST has not been run")),
        }
    }

    fn iffalse(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("iffalse", &args, 1)?;
        match self.test_result {
            Some(false) => {
                let list = list_input(&args[0], "IFFALSE")?;
                match self.execute_instruction_list(list)? {
                    ControlFlow::None => Ok(PrimitiveResult::NoValue),
                    control => Ok(PrimitiveResult::Control(control)),
                }
            }
            Some(true) => Ok(PrimitiveResult::NoValue),
            None => Err(VmError::new("TEST has not been run")),
        }
    }

    fn wait(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("wait", &args, 1)?;
        let sixtieths = number_input(&args[0], &self.interner)?.max(0.0);
        thread::sleep(Duration::from_secs_f64(sixtieths / 60.0));
        Ok(PrimitiveResult::NoValue)
    }

    fn catch(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("catch", &args, 2)?;
        let tag = args[0].clone();
        let list = list_input(&args[1], "CATCH")?;

        match self.execute_instruction_list(list) {
            Ok(control) => match control {
                ControlFlow::Throw {
                    tag: thrown_tag,
                    value,
                } if thrown_tag.equalp(&tag, &self.interner) => Ok(PrimitiveResult::Value(value)),
                ControlFlow::None | ControlFlow::Stop => Ok(PrimitiveResult::NoValue),
                ControlFlow::Output(value) => Ok(PrimitiveResult::Value(value)),
                ControlFlow::Throw { tag, value } => {
                    Ok(PrimitiveResult::Control(ControlFlow::Throw { tag, value }))
                }
            },
            Err(error) if is_error_catch_tag(&tag, &self.interner) => Ok(PrimitiveResult::Value(
                Value::word(&mut self.interner, error.message),
            )),
            Err(error) => Err(error),
        }
    }

    fn throw(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("throw", &args, 2)?;
        Ok(PrimitiveResult::Control(ControlFlow::Throw {
            tag: args[0].clone(),
            value: args[1].clone(),
        }))
    }

    fn error(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("error", &args, 0)?;
        let message = self.last_error.clone().unwrap_or_default();
        Ok(PrimitiveResult::Value(Value::word(
            &mut self.interner,
            message,
        )))
    }

    fn pause(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pause", &args, 0)?;
        Err(VmError::new("PAUSE debugger is not implemented yet"))
    }

    fn continue_(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("continue", &args, 0)?;
        Err(VmError::new("CONTINUE debugger is not implemented yet"))
    }

    fn turtle_forward(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("forward", &args, 1)?;
        let distance = number_input(&args[0], &self.interner)?;
        self.turtle.forward(distance);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_back(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("back", &args, 1)?;
        let distance = number_input(&args[0], &self.interner)?;
        self.turtle.back(distance);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_left(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("left", &args, 1)?;
        self.turtle.left(number_input(&args[0], &self.interner)?);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_right(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("right", &args, 1)?;
        self.turtle.right(number_input(&args[0], &self.interner)?);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setxy(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setxy", &args, 2)?;
        let x = number_input(&args[0], &self.interner)?;
        let y = number_input(&args[1], &self.interner)?;
        self.turtle.set_xy(x, y);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setpos(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setpos", &args, 1)?;
        let point = point_input(&args[0], &self.interner)?;
        self.turtle.set_pos(point);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setheading(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setheading", &args, 1)?;
        self.turtle
            .set_heading(number_input(&args[0], &self.interner)?);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_home(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("home", &args, 0)?;
        self.turtle.home();
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_clearscreen(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("clearscreen", &args, 0)?;
        self.turtle.clearscreen();
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_penup(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("penup", &args, 0)?;
        self.turtle.pen_up();
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_pendown(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pendown", &args, 0)?;
        self.turtle.pen_down();
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setpencolor(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setpencolor", &args, 1)?;
        self.turtle
            .set_pen_color(number_input(&args[0], &self.interner)? as u32);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setpensize(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setpensize", &args, 1)?;
        self.turtle
            .set_pen_size(number_input(&args[0], &self.interner)?);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_hide(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("hideturtle", &args, 0)?;
        self.turtle.hide_turtle();
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_show(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("showturtle", &args, 0)?;
        self.turtle.show_turtle();
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_pos(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pos", &args, 0)?;
        let position = self.turtle.state().position;
        Ok(PrimitiveResult::Value(Value::List(List::from_values([
            Value::number(position.x),
            Value::number(position.y),
        ]))))
    }

    fn turtle_heading(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("heading", &args, 0)?;
        Ok(PrimitiveResult::Value(Value::number(
            self.turtle.state().heading,
        )))
    }

    fn turtle_xcor(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("xcor", &args, 0)?;
        Ok(PrimitiveResult::Value(Value::number(
            self.turtle.state().position.x,
        )))
    }

    fn turtle_ycor(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("ycor", &args, 0)?;
        Ok(PrimitiveResult::Value(Value::number(
            self.turtle.state().position.y,
        )))
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

fn property_key_input(value: &Value, interner: &Interner) -> Result<String, VmError> {
    match value {
        Value::Word(symbol) => Ok(interner.canonical_spelling(*symbol).to_string()),
        Value::Number(_) => Ok(value.show(interner)),
        Value::List(_) | Value::Array(_) => Err(VmError::new(format!(
            "{} is not a property-list key",
            value.show(interner)
        ))),
    }
}

fn source_text_input(value: &Value, interner: &Interner) -> String {
    match value {
        Value::Word(symbol) => interner.spelling(*symbol).to_string(),
        _ => value.show(interner),
    }
}

fn token_to_data_value(kind: TokenKind, interner: &mut Interner) -> Option<Value> {
    match kind {
        TokenKind::Word(word) => Some(number_or_word_value(interner, word)),
        TokenKind::QuotedWord(word) => Some(Value::word(interner, word)),
        TokenKind::ColonWord(word) => Some(Value::word(interner, format!(":{word}"))),
        TokenKind::Infix(op) => Some(Value::word(interner, op.to_string())),
        TokenKind::LBracket => Some(Value::word(interner, "[")),
        TokenKind::RBracket => Some(Value::word(interner, "]")),
        TokenKind::LParen => Some(Value::word(interner, "(")),
        TokenKind::RParen => Some(Value::word(interner, ")")),
        TokenKind::LBrace => Some(Value::word(interner, "{")),
        TokenKind::RBrace => Some(Value::word(interner, "}")),
    }
}

fn number_or_word_value(interner: &mut Interner, word: String) -> Value {
    match word.parse::<f64>() {
        Ok(number) if number.is_finite() => Value::number(number),
        _ => Value::word(interner, word),
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

fn list_values(list: &List) -> Vec<Value> {
    list.iter().cloned().collect()
}

fn result_value(result: RunResult) -> Result<Value, VmError> {
    match result.control {
        ControlFlow::None => result
            .stack
            .last()
            .cloned()
            .ok_or_else(|| VmError::new("instruction list did not output a value")),
        ControlFlow::Output(value) => Ok(value),
        ControlFlow::Stop => Err(VmError::new("instruction list stopped without output")),
        ControlFlow::Throw { tag, value } => Ok(Value::List(List::from_values([tag, value]))),
    }
}

fn point_input(value: &Value, interner: &Interner) -> Result<Point, VmError> {
    let list = list_input(value, "SETPOS")?;
    let x = list
        .item(1)
        .ok_or_else(|| VmError::new("SETPOS requires a two-number list"))?;
    let y = list
        .item(2)
        .ok_or_else(|| VmError::new("SETPOS requires a two-number list"))?;
    if list.len() != 2 {
        return Err(VmError::new("SETPOS requires a two-number list"));
    }
    Ok(Point::new(
        number_input(x, interner)?,
        number_input(y, interner)?,
    ))
}

fn sentence_part(value: &Value, values: &mut Vec<Value>) {
    match value {
        Value::List(list) => values.extend(list.iter().cloned()),
        Value::Array(array) => values.extend(array.to_list().iter().cloned()),
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
        Value::Array(array) => array
            .to_list()
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
        Value::Array(array) => !array.is_empty(),
    }
}

fn is_error_catch_tag(value: &Value, interner: &Interner) -> bool {
    matches!(
        value,
        Value::Word(symbol) if interner.canonical_spelling(*symbol).eq("error")
    )
}

fn procedure_text(
    procedure: &Procedure,
    interner: &mut Interner,
    include_end: bool,
) -> Result<List, VmError> {
    let mut lines = vec![procedure_header_line(procedure, interner)];
    for line in procedure.body_source().lines() {
        lines.push(parse_source_line(line, interner)?);
    }
    if include_end {
        lines.push(parse_source_line("end", interner)?);
    }
    Ok(List::from_values(lines.into_iter().map(Value::List)))
}

fn procedure_header_line(procedure: &Procedure, interner: &mut Interner) -> List {
    let mut values = vec![Value::word(interner, "to"), Value::Word(procedure.name())];
    values.extend(
        procedure
            .params()
            .iter()
            .map(|param| Value::word(interner, format!(":{}", interner.spelling(*param)))),
    );
    List::from_values(values)
}

fn parse_source_line(line: &str, interner: &mut Interner) -> Result<List, VmError> {
    let tokens = lex(line).map_err(|error| VmError::new(error.to_string()))?;
    let values = tokens
        .into_iter()
        .filter_map(|token| token_to_data_value(token.kind, interner))
        .collect::<Vec<_>>();
    Ok(List::from_values(values))
}

fn is_primitive_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "sum"
            | "+"
            | "difference"
            | "-"
            | "product"
            | "*"
            | "quotient"
            | "/"
            | "remainder"
            | "and"
            | "or"
            | "not"
            | "equalp"
            | "equal?"
            | "emptyp"
            | "empty?"
            | "memberp"
            | "member?"
            | "first"
            | "butfirst"
            | "bf"
            | "last"
            | "butlast"
            | "bl"
            | "fput"
            | "lput"
            | "sentence"
            | "se"
            | "list"
            | "word"
            | "count"
            | "item"
            | "print"
            | "pr"
            | "show"
            | "type"
            | "readlist"
            | "rl"
            | "make"
            | "name"
            | "thing"
            | "local"
            | "definedp"
            | "defined?"
            | "primitivep"
            | "primitive?"
            | "pprop"
            | "gprop"
            | "remprop"
            | "plist"
            | "array"
            | "setitem"
            | "listtoarray"
            | "arraytolist"
            | "repeat"
            | "if"
            | "ifelse"
            | "run"
            | "runresult"
            | "parse"
            | "runparse"
            | "apply"
            | "foreach"
            | "map"
            | "filter"
            | "reduce"
            | "repcount"
            | "test"
            | "iftrue"
            | "ift"
            | "iffalse"
            | "iff"
            | "wait"
            | "catch"
            | "throw"
            | "error"
            | "pause"
            | "continue"
            | "forward"
            | "fd"
            | "back"
            | "bk"
            | "left"
            | "lt"
            | "right"
            | "rt"
            | "setxy"
            | "setpos"
            | "setheading"
            | "seth"
            | "home"
            | "clearscreen"
            | "cs"
            | "penup"
            | "pu"
            | "pendown"
            | "pd"
            | "setpencolor"
            | "setpc"
            | "setpensize"
            | "hideturtle"
            | "ht"
            | "showturtle"
            | "st"
            | "pos"
            | "heading"
            | "xcor"
            | "ycor"
            | "output"
            | "op"
            | "stop"
    )
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
                if is_placeholder_word(spelling) {
                    format!(":{spelling}")
                } else if spelling.starts_with(':') {
                    spelling.to_string()
                } else if arities.get(spelling).is_some() || is_operator_word(spelling) {
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

fn is_placeholder_word(text: &str) -> bool {
    text == "?"
        || text
            .strip_prefix('?')
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turtle::{Point, TurtleEvent};

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
    fn make_updates_local_bindings_instead_of_shadowed_globals() {
        let (result, vm) = run("make \"x 1
             to change_local
             local \"x
             make \"x 5
             print :x
             end
             change_local
             print :x")
        .unwrap();
        assert_eq!(result.output, "5\n1\n");
        assert_eq!(vm.env().get("x"), Some(&Value::number(1.0)));
    }

    #[test]
    fn make_can_rebind_procedure_arguments() {
        let (result, _) = run("to bump :x
             make \"x sum :x 1
             print :x
             end
             bump 4")
        .unwrap();
        assert_eq!(result.output, "5\n");
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

    #[test]
    fn template_primitives_map_filter_reduce_foreach_apply() {
        let (result, _) = run("print map [sum ? 1] [1 2 3] \
             print filter [? > 1] [0 1 2 3] \
             print reduce [sum ?1 ?2] [1 2 3 4] \
             foreach [print ?] [a b] \
             print apply \"sum [10 20] \
             print runresult [sum 7 8]")
        .unwrap();
        assert_eq!(result.output, "[2 3 4]\n[2 3]\n10\na\nb\n30\n15\n");
    }

    #[test]
    fn library_control_structures_run_over_instruction_lists() {
        let (result, _) = run("make \"x 0 \
             while [:x < 3] [make \"x sum :x 1] \
             print :x \
             make \"y 0 \
             until [:y = 2] [make \"y sum :y 1] \
             print :y \
             make \"z 0 \
             do.while [make \"z sum :z 1] [:z < 2] \
             print :z")
        .unwrap();
        assert_eq!(result.output, "3\n2\n2\n");
    }

    #[test]
    fn cond_case_and_for_are_available_by_default() {
        let (result, _) = run("make \"acc [] \
             for [i 1 5 2] [make \"acc lput :i :acc] \
             print :acc \
             cond [[[false] [print \"bad]] [[true] [print \"good]]] \
             case 2 [[[1] [print \"one]] [[2 3] [print \"small]] [else [print \"other]]]")
        .unwrap();
        assert_eq!(result.output, "[1 3 5]\ngood\nsmall\n");
    }

    #[test]
    fn array_primitives() {
        let (result, _) = run("make \"a array 3 \
             setitem 2 :a \"middle \
             print item 2 :a \
             print count :a \
             print arraytolist :a \
             print arraytolist listtoarray [x y]")
        .unwrap();
        assert_eq!(result.output, "middle\n3\n[[] middle []]\n[x y]\n");
    }

    #[test]
    fn property_list_primitives() {
        let (result, vm) = run("pprop \"animal \"legs 4 \
             pprop \"animal \"sound \"woof \
             print gprop \"animal \"legs \
             print plist \"animal \
             remprop \"animal \"legs \
             print gprop \"animal \"legs")
        .unwrap();
        assert_eq!(result.output, "4\n[legs 4 sound woof]\n[]\n");
        assert!(vm.property_lists().contains_key("animal"));
    }

    #[test]
    fn workspace_predicates_report_defined_and_primitive_procedures() {
        let (result, _) = run("to square :x
             output product :x :x
             end
             print definedp \"square
             print definedp \"sum
             print primitivep \"sum
             print primitivep \"square
             print primitive? \"fd")
        .unwrap();
        assert_eq!(result.output, "true\nfalse\ntrue\nfalse\ntrue\n");
    }

    #[test]
    fn text_and_fulltext_expose_workspace_procedure_source() {
        let (result, _) = run("to square :x
             output product :x :x
             end
             print text \"square
             print fulltext \"square")
        .unwrap();
        assert_eq!(
            result.output,
            "[[to square :x] [output product :x :x]]\n[[to square :x] [output product :x :x] [end]]\n"
        );
    }

    #[test]
    fn parse_and_runparse() {
        let (result, _) = run("print parse \"|sum 1 2| print runparse \"|sum 3 4|").unwrap();
        assert_eq!(result.output, "[sum 1 2]\n7\n");
    }

    #[test]
    fn test_iftrue_and_iffalse_use_last_test_result() {
        let (result, _) = run("test equalp 1 1 iftrue [print \"yes] iffalse [print \"no] \
             test equalp 1 2 iftrue [print \"bad] iffalse [print \"good]")
        .unwrap();
        assert_eq!(result.output, "yes\ngood\n");
    }

    #[test]
    fn catch_returns_matching_throw_value() {
        let (result, _) = run("print catch \"tag [throw \"tag \"value print \"bad]").unwrap();
        assert_eq!(result.output, "value\n");
    }

    #[test]
    fn uncaught_throw_propagates_as_control_flow() {
        let (result, vm) = run("throw \"outer 17 print \"bad").unwrap();
        let ControlFlow::Throw { tag, value } = result.control else {
            panic!("expected uncaught THROW");
        };
        assert_eq!(tag.show(vm.interner()), "outer");
        assert_eq!(value, Value::number(17.0));
        assert_eq!(result.output, "");
    }

    #[test]
    fn wait_zero_is_noop_and_error_outputs_last_error_word() {
        let (result, _) = run("wait 0 print error").unwrap();
        assert_eq!(result.output, "\n");
    }

    #[test]
    fn top_level_unused_value_is_an_error() {
        let mut vm = Vm::new();
        let error = vm.eval_source("sum 2 3 print 9").unwrap_err();
        assert_eq!(error.message, "You don't say what to do with 5");
        assert_eq!(vm.output(), "");
    }

    #[test]
    fn run_rejects_unused_values_inside_instruction_lists() {
        let mut vm = Vm::new();
        let error = vm.eval_source("run [sum 2 3 print 9]").unwrap_err();
        assert_eq!(error.message, "You don't say what to do with 5");
        assert_eq!(vm.output(), "");
    }

    #[test]
    fn error_primitive_reports_last_failure_message() {
        let mut vm = Vm::new();
        let error = vm.eval_source("print").unwrap_err();
        assert!(error.message.starts_with("not enough inputs to print"));
        let result = vm.eval_source("print error").unwrap();
        assert!(result.output.starts_with("not enough inputs to print"));
    }

    #[test]
    fn catch_error_returns_last_failure_message() {
        let mut vm = Vm::new();
        let result = vm.eval_source("print catch \"error [print]").unwrap();
        assert!(result.output.starts_with("not enough inputs to print"));
    }

    #[test]
    fn procedure_used_as_input_reports_missing_output() {
        let mut vm = Vm::new();
        let error = vm
            .eval_source(
                "to noop :x
             print :x
             end
             print noop 5",
            )
            .unwrap_err();
        assert!(error.message.starts_with("noop didn't output a value"));
    }

    #[test]
    fn turtle_motion_primitives_update_state_and_record_lines() {
        let (result, vm) = run("fd 100 rt 90 fd 50 print pos print heading").unwrap();
        assert_eq!(result.output, "[50 100]\n90\n");
        assert_eq!(vm.turtle().state().position, Point::new(50.0, 100.0));
        assert_eq!(vm.turtle().state().heading, 90.0);
        let line_count = vm
            .turtle()
            .backend()
            .events()
            .iter()
            .filter(|event| matches!(event, TurtleEvent::Line { .. }))
            .count();
        assert_eq!(line_count, 2);
    }

    #[test]
    fn pen_and_visibility_primitives() {
        let (result, vm) =
            run("pu fd 10 pd setpc 3 setpensize 4 fd 5 ht st print xcor print ycor").unwrap();
        assert_eq!(result.output, "0\n15\n");
        let state = vm.turtle().state();
        assert!(state.pen_down);
        assert!(state.visible);
        assert_eq!(state.pen_color, 3);
        assert_eq!(state.pen_size, 4.0);
        let line_count = vm
            .turtle()
            .backend()
            .events()
            .iter()
            .filter(|event| matches!(event, TurtleEvent::Line { .. }))
            .count();
        assert_eq!(line_count, 1);
    }

    #[test]
    fn setpos_setxy_home_and_clearscreen() {
        let (_, mut vm) = run("setpos [10 20] setxy 30 40 home cs").unwrap();
        assert_eq!(vm.turtle().state().position, Point::new(0.0, 0.0));
        assert_eq!(vm.turtle().state().heading, 0.0);
        assert!(vm
            .turtle()
            .backend()
            .events()
            .iter()
            .any(|event| matches!(event, TurtleEvent::Clear)));
        vm.turtle_mut().backend_mut().clear_events();
        assert!(vm.turtle().backend().events().is_empty());
    }
}
