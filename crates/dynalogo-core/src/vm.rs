//! Stack VM for DynaLOGO bytecode.
//!
//! This is the initial v0.1 executor. It establishes the pieces that later
//! procedure support will build on: a shared interner, dynamic-scope frame
//! stack, primitive dispatch, bytecode stack execution, and `OUTPUT`/`STOP`
//! control signals.

use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::f64::consts::PI;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
struct InputStream {
    path: PathBuf,
    content: String,
    cursor: usize,
}

impl InputStream {
    fn open(path: PathBuf) -> Result<Self, VmError> {
        let content = fs::read_to_string(&path)
            .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        Ok(Self {
            path,
            content,
            cursor: 0,
        })
    }

    fn read_char(&mut self) -> Option<String> {
        let tail = self.content.get(self.cursor..)?;
        let ch = tail.chars().next()?;
        self.cursor += ch.len_utf8();
        Some(ch.to_string())
    }

    fn read_line(&mut self) -> Option<String> {
        let tail = self.content.get(self.cursor..)?;
        if tail.is_empty() {
            return None;
        }
        if let Some(newline) = tail.find('\n') {
            let mut line = &tail[..newline];
            self.cursor += newline + 1;
            if let Some(stripped) = line.strip_suffix('\r') {
                line = stripped;
            }
            Some(line.to_string())
        } else {
            self.cursor = self.content.len();
            Some(tail.strip_suffix('\r').unwrap_or(tail).to_string())
        }
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
    read_stream: Option<InputStream>,
    current_read_managed: bool,
    read_streams: HashMap<String, InputStream>,
    current_write: Option<String>,
    write_streams: HashSet<String>,
    dribble: Option<PathBuf>,
    test_result: Option<bool>,
    last_error: Option<String>,
    random_seed: u64,
    edit_buffer: Option<EditSession>,
    editor_override: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct EditContents {
    procedures: Vec<String>,
    variables: Vec<String>,
    plists: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct EditSession {
    contents: EditContents,
    text: String,
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
            read_stream: None,
            current_read_managed: false,
            read_streams: HashMap::new(),
            current_write: None,
            write_streams: HashSet::new(),
            dribble: None,
            test_result: None,
            last_error: None,
            random_seed: 0x4d595df4d0f33173,
            edit_buffer: None,
            editor_override: None,
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

    /// Override the command EDIT/ED launch instead of reading `$EDITOR`.
    ///
    /// Lets embedding frontends (and tests) supply a deterministic editor
    /// command without mutating the process-wide environment.
    pub fn set_editor_command(&mut self, command: impl Into<String>) {
        self.editor_override = Some(command.into());
    }

    pub fn output(&self) -> &str {
        &self.output
    }

    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    fn write_output_fragment(&mut self, text: &str) -> Result<(), VmError> {
        if let Some(name) = self.current_write.clone() {
            let path = PathBuf::from(&name);
            let mut current = fs::read_to_string(&path).unwrap_or_default();
            current.push_str(text);
            fs::write(&path, current)
                .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        } else {
            self.output.push_str(text);
            if let Some(path) = self.dribble.clone() {
                let mut current = fs::read_to_string(&path).unwrap_or_default();
                current.push_str(text);
                fs::write(&path, current)
                    .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
            }
        }
        Ok(())
    }

    fn write_output_line(&mut self, text: &str) -> Result<(), VmError> {
        self.write_output_fragment(text)?;
        self.write_output_fragment("\n")
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
            "abs" => self.abs(args),
            "int" => self.int(args),
            "round" => self.round(args),
            "sqrt" => self.sqrt(args),
            "sin" => self.sin(args),
            "cos" => self.cos(args),
            "tan" => self.tan(args),
            "random" => self.random(args),
            "rerandom" => self.rerandom(args),
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
            "which" => self.which(args),
            "dot" => self.dot(args),
            "before" => self.before(args),
            "insert" => self.insert(args),
            "sort" => self.sort(args),
            "supersort" => self.supersort(args),
            "print" | "pr" => self.print(args),
            "show" => self.show(args),
            "type" => self.r#type(args),
            "load" => self.load(args),
            "save" => self.save(args),
            "setread" => self.setread(args),
            "setwrite" => self.setwrite(args),
            "readchar" | "rc" => self.readchar(args),
            "readlist" | "rl" => self.readlist(args),
            "readword" | "rw" => self.readword(args),
            "openread" => self.openread(args),
            "openwrite" => self.openwrite(args),
            "openappend" => self.openappend(args),
            "close" => self.close(args),
            "reader" => self.reader(args),
            "writer" => self.writer(args),
            "dribble" => self.dribble_command(args),
            "nodribble" => self.nodribble(args),
            "make" | "name" => self.make(args),
            "thing" => self.thing(args),
            "local" => self.local(args),
            "namep" => self.namep(args),
            "wordp" => self.wordp(args),
            "listp" => self.listp(args),
            "numberp" => self.numberp(args),
            "intp" => self.intp(args),
            "decimalp" => self.decimalp(args),
            "definedp" | "defined?" => self.definedp(args),
            "primitivep" | "primitive?" => self.primitivep(args),
            "text" => self.text(args),
            "fulltext" => self.fulltext(args),
            "copydef" => self.copydef(args),
            "define" => self.define_from_data(args),
            "edit" | "ed" => self.edit(args),
            "po" => self.po(args),
            "poall" => self.poall(args),
            "pons" => self.pons(args),
            "pops" => self.pops(args),
            "pots" => self.pots(args),
            ".primitives" => self.primitives_command(args),
            "erase" | "er" => self.erase(args),
            "ern" => self.ern(args),
            "erns" => self.erns(args),
            "erps" => self.erps(args),
            "erall" => self.erall(args),
            "pprop" => self.pprop(args),
            "gprop" => self.gprop(args),
            "remprop" => self.remprop(args),
            "plist" => self.plist(args),
            "array" => self.array(args),
            "setitem" => self.setitem(args),
            "listtoarray" => self.listtoarray(args),
            "arraytolist" => self.arraytolist(args),
            "repeat" => self.repeat(args),
            "forever" => self.forever(args),
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
            "cascade" => self.cascade(args),
            "cascade.2" => self.cascade2(args),
            "transfer" => self.transfer(args),
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
            "setx" => self.turtle_setx(args),
            "sety" => self.turtle_sety(args),
            "setpos" => self.turtle_setpos(args),
            "setheading" | "seth" => self.turtle_setheading(args),
            "home" => self.turtle_home(args),
            "clearscreen" | "cs" => self.turtle_clearscreen(args),
            "penup" | "pu" => self.turtle_penup(args),
            "pendown" | "pd" => self.turtle_pendown(args),
            "setpencolor" | "setpc" => self.turtle_setpencolor(args),
            "setpensize" => self.turtle_setpensize(args),
            "hideturtle" | "ht" => self.turtle_hide(args),
            "init.turtle" => self.init_turtle(args),
            "showturtle" | "st" => self.turtle_show(args),
            "shownp" => self.turtle_shownp(args),
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

    fn unary_number_op(
        &mut self,
        name: &str,
        args: Vec<Value>,
        op: impl FnOnce(f64) -> f64,
    ) -> Result<PrimitiveResult, VmError> {
        expect_arity(name, &args, 1)?;
        let value = number_input(&args[0], &self.interner)?;
        Ok(PrimitiveResult::Value(Value::number(op(value))))
    }

    fn abs(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        self.unary_number_op("abs", args, f64::abs)
    }

    fn int(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        self.unary_number_op("int", args, f64::trunc)
    }

    fn round(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        self.unary_number_op("round", args, f64::round)
    }

    fn sqrt(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        self.unary_number_op("sqrt", args, f64::sqrt)
    }

    fn sin(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        self.unary_number_op("sin", args, |value| (value * PI / 180.0).sin())
    }

    fn cos(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        self.unary_number_op("cos", args, |value| (value * PI / 180.0).cos())
    }

    fn tan(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        self.unary_number_op("tan", args, |value| (value * PI / 180.0).tan())
    }

    fn random(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("random", &args, 1)?;
        let upper = number_input(&args[0], &self.interner)?;
        if upper <= 0.0 {
            return Err(VmError::new("RANDOM input must be positive"));
        }
        let upper = upper.floor() as u64;
        let value = self.next_random_u64() % upper;
        Ok(PrimitiveResult::Value(Value::number(value as f64)))
    }

    fn rerandom(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("rerandom", &args, 0)?;
        self.random_seed = 0x4d595df4d0f33173;
        Ok(PrimitiveResult::NoValue)
    }

    fn next_random_u64(&mut self) -> u64 {
        self.random_seed = self
            .random_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.random_seed
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
            Value::Word(symbol) | Value::BareWord(symbol) => {
                self.interner.spelling(*symbol).is_empty()
            }
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
            Value::Word(symbol) | Value::BareWord(symbol) => self
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
            Value::Word(symbol) | Value::BareWord(symbol) => {
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
            Value::Word(symbol) | Value::BareWord(symbol) => {
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
            Value::Word(symbol) | Value::BareWord(symbol) => {
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
            Value::Word(symbol) | Value::BareWord(symbol) => {
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
            Value::Word(symbol) | Value::BareWord(symbol) => {
                self.interner.spelling(*symbol).chars().count()
            }
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
            Value::Word(symbol) | Value::BareWord(symbol) => {
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

    fn which(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("which", &args, 2)?;
        let values = match &args[1] {
            Value::List(list) => list.iter().cloned().collect::<Vec<_>>(),
            Value::Array(array) => array.to_list().iter().cloned().collect::<Vec<_>>(),
            _ => {
                return Err(VmError::new(format!(
                    "{} is not a list",
                    args[1].show(&self.interner)
                )))
            }
        };
        let position = values
            .iter()
            .position(|value| args[0].equalp(value, &self.interner))
            .map(|index| index as f64 + 1.0)
            .unwrap_or(0.0);
        Ok(PrimitiveResult::Value(Value::number(position)))
    }

    fn dot(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("dot", &args, 1)?;
        let target = point_input(&args[0], &self.interner)?;
        let state = self.turtle.state();

        self.turtle.pen_up();
        self.turtle.set_pos(target);
        self.turtle.pen_down();
        self.turtle.forward(0.0);
        self.turtle.pen_up();
        self.turtle.set_pos(state.position);
        self.turtle.set_heading(state.heading);
        self.turtle.set_pen_color(state.pen_color);
        self.turtle.set_pen_size(state.pen_size);
        if state.pen_down {
            self.turtle.pen_down();
        }
        if state.visible {
            self.turtle.show_turtle();
        } else {
            self.turtle.hide_turtle();
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn before(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("before", &args, 2)?;
        let a = source_text_input(&args[0], &self.interner);
        let b = source_text_input(&args[1], &self.interner);
        Ok(PrimitiveResult::Value(self.logo_bool(before_text(&a, &b))))
    }

    fn insert(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("insert", &args, 2)?;
        let tree = match &args[1] {
            Value::List(list) => list.clone(),
            _ => {
                return Err(VmError::new(format!(
                    "{} is not a sort tree",
                    args[1].show(&self.interner)
                )))
            }
        };
        Ok(PrimitiveResult::Value(Value::List(insert_sorted_tree(
            args[0].clone(),
            &tree,
            &self.interner,
        )?)))
    }

    fn sort(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("sort", &args, 2)?;
        let values = list_values(list_input(&args[0], "SORT")?);
        let mut tree = list_input(&args[1], "SORT")?.clone();
        for value in values {
            tree = insert_sorted_tree(value, &tree, &self.interner)?;
        }
        Ok(PrimitiveResult::Value(Value::List(tree)))
    }

    fn supersort(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("supersort", &args, 1)?;
        let tree = list_input(&args[0], "SUPERSORT")?.clone();
        Ok(PrimitiveResult::Value(Value::List(flatten_sorted_tree(
            &tree,
        )?)))
    }

    fn print(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("print", &args, 1)?;
        self.write_output_line(&args[0].show(&self.interner))?;
        Ok(PrimitiveResult::NoValue)
    }

    fn show(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("show", &args, 1)?;
        self.write_output_line(&args[0].show(&self.interner))?;
        Ok(PrimitiveResult::NoValue)
    }

    fn r#type(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("type", &args, 1)?;
        self.write_output_fragment(&args[0].show(&self.interner))?;
        Ok(PrimitiveResult::NoValue)
    }

    fn load(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("load", &args, 1)?;
        let path = PathBuf::from(source_text_input(&args[0], &self.interner));
        let source = fs::read_to_string(&path)
            .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        let result = self.eval_source(&source)?;
        match result.control {
            ControlFlow::None => Ok(PrimitiveResult::NoValue),
            control => Ok(PrimitiveResult::Control(control)),
        }
    }

    fn save(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("save", &args, 1)?;
        let path = PathBuf::from(source_text_input(&args[0], &self.interner));
        let procedures = self.visible_workspace_procedures();
        let mut source = String::new();
        for (index, procedure) in procedures.iter().enumerate() {
            source.push_str("to ");
            source.push_str(self.interner.spelling(procedure.name()));
            for param in procedure.params() {
                source.push(' ');
                source.push(':');
                source.push_str(self.interner.spelling(*param));
            }
            source.push('\n');
            if !procedure.body_source().is_empty() {
                source.push_str(procedure.body_source());
                if !procedure.body_source().ends_with('\n') {
                    source.push('\n');
                }
            }
            source.push_str("end\n");
            if index + 1 < procedures.len() {
                source.push('\n');
            }
        }
        fs::write(&path, source)
            .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        Ok(PrimitiveResult::NoValue)
    }

    fn park_current_read(&mut self) {
        if self.current_read_managed {
            if let Some(stream) = self.read_stream.take() {
                let key = stream.path.to_string_lossy().to_string();
                self.read_streams.insert(key, stream);
            }
        } else {
            self.read_stream = None;
        }
        self.current_read_managed = false;
    }

    fn setread(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setread", &args, 1)?;
        if args[0].is_empty_list() {
            self.park_current_read();
        } else {
            let key = source_text_input(&args[0], &self.interner);
            self.park_current_read();
            if let Some(stream) = self.read_streams.remove(&key) {
                self.read_stream = Some(stream);
                self.current_read_managed = true;
            } else {
                self.read_stream = Some(InputStream::open(PathBuf::from(&key))?);
                self.current_read_managed = false;
            }
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn setwrite(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setwrite", &args, 1)?;
        if args[0].is_empty_list() {
            self.current_write = None;
        } else {
            let key = source_text_input(&args[0], &self.interner);
            if !self.write_streams.contains(&key) {
                let path = PathBuf::from(&key);
                fs::write(&path, "")
                    .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
            }
            self.current_write = Some(key);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn openread(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("openread", &args, 1)?;
        let key = source_text_input(&args[0], &self.interner);
        let stream = InputStream::open(PathBuf::from(&key))?;
        self.read_streams.insert(key, stream);
        Ok(PrimitiveResult::NoValue)
    }

    fn openwrite(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("openwrite", &args, 1)?;
        let key = source_text_input(&args[0], &self.interner);
        let path = PathBuf::from(&key);
        fs::write(&path, "")
            .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        self.write_streams.insert(key);
        Ok(PrimitiveResult::NoValue)
    }

    fn openappend(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("openappend", &args, 1)?;
        let key = source_text_input(&args[0], &self.interner);
        let path = PathBuf::from(&key);
        if !path.exists() {
            fs::write(&path, "")
                .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        }
        self.write_streams.insert(key);
        Ok(PrimitiveResult::NoValue)
    }

    fn close(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("close", &args, 1)?;
        let key = source_text_input(&args[0], &self.interner);
        let mut closed = false;

        let active_read_matches = self
            .read_stream
            .as_ref()
            .map(|stream| stream.path.to_string_lossy() == key)
            .unwrap_or(false);
        if active_read_matches {
            self.read_stream = None;
            self.current_read_managed = false;
            closed = true;
        } else if self.read_streams.remove(&key).is_some() {
            closed = true;
        }

        if self.current_write.as_deref() == Some(key.as_str()) {
            self.current_write = None;
            closed = true;
        }
        if self.write_streams.remove(&key) {
            closed = true;
        }

        if closed {
            Ok(PrimitiveResult::NoValue)
        } else {
            Err(VmError::new(format!("CLOSE: {key} is not open")))
        }
    }

    fn reader(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("reader", &args, 0)?;
        let value = match &self.read_stream {
            Some(stream) => Value::word(&mut self.interner, stream.path.to_string_lossy()),
            None => Value::List(List::empty()),
        };
        Ok(PrimitiveResult::Value(value))
    }

    fn writer(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("writer", &args, 0)?;
        let value = match self.current_write.clone() {
            Some(name) => Value::word(&mut self.interner, name),
            None => Value::List(List::empty()),
        };
        Ok(PrimitiveResult::Value(value))
    }

    fn dribble_command(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("dribble", &args, 1)?;
        let path = PathBuf::from(source_text_input(&args[0], &self.interner));
        fs::write(&path, "")
            .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        self.dribble = Some(path);
        Ok(PrimitiveResult::NoValue)
    }

    fn nodribble(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("nodribble", &args, 0)?;
        self.dribble = None;
        Ok(PrimitiveResult::NoValue)
    }

    fn readchar(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("readchar", &args, 0)?;
        let stream = self
            .read_stream
            .as_mut()
            .ok_or_else(|| VmError::new("READCHAR is not connected to an input stream yet"))?;
        let ch = stream.read_char().ok_or_else(|| {
            VmError::new(format!(
                "READCHAR reached end of input stream {}",
                stream.path.display()
            ))
        })?;
        Ok(PrimitiveResult::Value(Value::word(&mut self.interner, ch)))
    }

    fn readlist(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("readlist", &args, 0)?;
        let stream = self
            .read_stream
            .as_mut()
            .ok_or_else(|| VmError::new("READLIST is not connected to an input stream yet"))?;
        let line = stream.read_line().ok_or_else(|| {
            VmError::new(format!(
                "READLIST reached end of input stream {}",
                stream.path.display()
            ))
        })?;
        Ok(PrimitiveResult::Value(Value::List(parse_source_line(
            &line,
            &mut self.interner,
        )?)))
    }

    fn readword(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("readword", &args, 0)?;
        let stream = self
            .read_stream
            .as_mut()
            .ok_or_else(|| VmError::new("READWORD is not connected to an input stream yet"))?;
        let line = stream.read_line().ok_or_else(|| {
            VmError::new(format!(
                "READWORD reached end of input stream {}",
                stream.path.display()
            ))
        })?;
        Ok(PrimitiveResult::Value(Value::word(
            &mut self.interner,
            line,
        )))
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

    fn namep(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("namep", &args, 1)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(self.env.get(&name).is_some()),
        ))
    }

    fn wordp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("wordp", &args, 1)?;
        Ok(PrimitiveResult::Value(self.logo_bool(matches!(
            args[0],
            Value::Word(_) | Value::BareWord(_) | Value::Number(_)
        ))))
    }

    fn listp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("listp", &args, 1)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(matches!(args[0], Value::List(_))),
        ))
    }

    fn numberp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("numberp", &args, 1)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(args[0].as_number(&self.interner).is_some()),
        ))
    }

    fn intp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("intp", &args, 1)?;
        let Some(number) = args[0].as_number(&self.interner) else {
            return Ok(PrimitiveResult::Value(self.logo_bool(false)));
        };
        Ok(PrimitiveResult::Value(
            self.logo_bool(LogoNumber::new(number).is_integerish()),
        ))
    }

    fn decimalp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("decimalp", &args, 1)?;
        let Some(number) = args[0].as_number(&self.interner) else {
            return Ok(PrimitiveResult::Value(self.logo_bool(false)));
        };
        Ok(PrimitiveResult::Value(
            self.logo_bool(number.is_finite() && number.fract() != 0.0),
        ))
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

    fn copydef(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("copydef", &args, 2)?;
        let new_name = variable_name_input(&args[0], &self.interner)?;
        let procedure = self.workspace_procedure(&args[1])?.clone();
        let params = procedure
            .params()
            .iter()
            .map(|param| self.interner.spelling(*param).to_string())
            .collect::<Vec<_>>();
        let body_source = procedure.body_source().to_string();
        self.define_procedure(new_name, params, &body_source)?;
        Ok(PrimitiveResult::NoValue)
    }

    fn define_from_data(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("define", &args, 3)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        let params = parameter_names_input(&args[1], &self.interner)?;
        let body_lines = define_body_input(&args[2], &self.interner, &self.arities)?;
        self.define_procedure(name, params, &body_lines.join("\n"))?;
        Ok(PrimitiveResult::NoValue)
    }

    fn po(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("po", &args, 1)?;
        let names = local_names(&args[0], &self.interner)?;
        for name in names {
            let procedure = self
                .procedures
                .get(&name.to_ascii_lowercase())
                .cloned()
                .ok_or_else(|| VmError::new(format!("I don't know how to {name}")))?;
            self.write_procedure_listing(&procedure, true, true)?;
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn poall(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("poall", &args, 0)?;
        let procedures = self.visible_workspace_procedures();
        for procedure in procedures {
            self.write_procedure_listing(&procedure, true, true)?;
        }
        self.write_variable_listing();
        Ok(PrimitiveResult::NoValue)
    }

    fn pons(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pons", &args, 0)?;
        self.write_variable_listing();
        Ok(PrimitiveResult::NoValue)
    }

    fn pops(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pops", &args, 0)?;
        for procedure in self.visible_workspace_procedures() {
            self.write_procedure_listing(&procedure, true, true)?;
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn pots(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pots", &args, 0)?;
        for procedure in self.visible_workspace_procedures() {
            self.write_procedure_listing(&procedure, false, false)?;
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn edit(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        if args.len() > 1 {
            return Err(VmError::new(format!(
                "edit expected 0 or 1 input(s), got {}",
                args.len()
            )));
        }
        let (contents, buffer_text) = match args.into_iter().next() {
            Some(value) => {
                let contents = contentslist_input(&value, &self.interner)?;
                let text = self.render_edit_buffer(&contents)?;
                (contents, text)
            }
            None => {
                let session = self
                    .edit_buffer
                    .clone()
                    .ok_or_else(|| VmError::new("EDIT needs a contents list to edit"))?;
                (session.contents, session.text)
            }
        };

        let edited = self.run_editor_on(&buffer_text)?;
        self.edit_buffer = Some(EditSession {
            contents,
            text: edited.clone(),
        });

        let result = self.eval_source(&edited)?;
        match result.control {
            ControlFlow::None => Ok(PrimitiveResult::NoValue),
            control => Ok(PrimitiveResult::Control(control)),
        }
    }

    fn render_edit_buffer(&mut self, contents: &EditContents) -> Result<String, VmError> {
        let mut buffer = String::new();
        for name in &contents.procedures {
            let key = name.to_ascii_lowercase();
            match self.procedures.get(&key).cloned() {
                Some(procedure) => {
                    buffer.push_str(&procedure_definition_text(&procedure, &self.interner))
                }
                None => {
                    buffer.push_str("to ");
                    buffer.push_str(name);
                    buffer.push_str("\nend\n");
                }
            }
            buffer.push('\n');
        }
        for name in &contents.variables {
            match self.env.get(name).cloned() {
                Some(Value::Array(_)) => {
                    buffer.push_str("; ");
                    buffer.push_str(name);
                    buffer.push_str(" is an array and cannot be edited as text\n");
                }
                Some(value) => {
                    buffer.push_str("make \"");
                    buffer.push_str(name);
                    buffer.push(' ');
                    buffer.push_str(&value_source_literal(&value, &self.interner));
                    buffer.push('\n');
                }
                None => {
                    buffer.push_str("make \"");
                    buffer.push_str(name);
                    buffer.push_str(" []\n");
                }
            }
        }
        for name in &contents.plists {
            let key = name.to_ascii_lowercase();
            if let Some(plist) = self.property_lists.get(&key) {
                let mut entries: Vec<_> = plist.iter().collect();
                entries.sort_by(|(a, _), (b, _)| a.cmp(b));
                for (prop, value) in entries {
                    buffer.push_str("pprop \"");
                    buffer.push_str(name);
                    buffer.push_str(" \"");
                    buffer.push_str(prop);
                    buffer.push(' ');
                    buffer.push_str(&value_source_literal(value, &self.interner));
                    buffer.push('\n');
                }
            }
        }
        Ok(buffer)
    }

    fn run_editor_on(&self, text: &str) -> Result<String, VmError> {
        let editor = resolve_editor_command(self.editor_override.as_deref(), env::var("EDITOR"))?;
        let mut parts = editor.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| VmError::new("EDITOR is set but empty"))?;
        let path = edit_temp_path();
        fs::write(&path, text)
            .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;

        Command::new(program)
            .args(parts)
            .arg(&path)
            .status()
            .map_err(|error| {
                VmError::new(format!("failed to launch EDITOR `{editor}`: {error}"))
            })?;

        let edited = fs::read_to_string(&path)
            .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        let _ = fs::remove_file(&path);
        Ok(edited)
    }

    fn primitives_command(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity(".primitives", &args, 0)?;
        self.output.push_str(&primitive_names().join(" "));
        self.output.push('\n');
        Ok(PrimitiveResult::NoValue)
    }

    fn erase(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("erase", &args, 1)?;
        for name in local_names(&args[0], &self.interner)? {
            let key = name.to_ascii_lowercase();
            if is_protected_workspace_procedure(&key) {
                continue;
            }
            self.procedures.remove(&key);
            self.arities.remove(&key);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn ern(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("ern", &args, 1)?;
        for name in local_names(&args[0], &self.interner)? {
            self.env.globals.remove(&name.to_ascii_lowercase());
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn erns(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("erns", &args, 0)?;
        self.env.globals.clear();
        Ok(PrimitiveResult::NoValue)
    }

    fn erps(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("erps", &args, 0)?;
        let removable = self
            .procedures
            .keys()
            .filter(|name| !is_protected_workspace_procedure(name))
            .cloned()
            .collect::<Vec<_>>();
        for name in removable {
            self.procedures.remove(&name);
            self.arities.remove(&name);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn erall(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("erall", &args, 0)?;
        self.erns(vec![])?;
        self.erps(vec![])?;
        self.property_lists.clear();
        Ok(PrimitiveResult::NoValue)
    }

    fn workspace_procedure(&self, value: &Value) -> Result<&Procedure, VmError> {
        let name = variable_name_input(value, &self.interner)?;
        self.procedures
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| VmError::new(format!("I don't know how to {name}")))
    }

    fn visible_workspace_procedures(&self) -> Vec<Procedure> {
        let mut procedures = self
            .procedures
            .iter()
            .filter(|(name, _)| !is_protected_workspace_procedure(name))
            .map(|(_, procedure)| procedure.clone())
            .collect::<Vec<_>>();
        procedures.sort_by_key(|procedure| {
            self.interner
                .canonical_spelling(procedure.name())
                .to_string()
        });
        procedures
    }

    fn write_variable_listing(&mut self) {
        let mut names = self.env.globals.keys().cloned().collect::<Vec<_>>();
        names.sort();
        for name in names {
            if let Some(value) = self.env.globals.get(&name) {
                self.output.push_str(&name);
                self.output.push(' ');
                self.output.push_str(&value.show(&self.interner));
                self.output.push('\n');
            }
        }
    }

    fn write_procedure_listing(
        &mut self,
        procedure: &Procedure,
        include_body: bool,
        include_end: bool,
    ) -> Result<(), VmError> {
        let lines = if include_body {
            procedure_text(procedure, &mut self.interner, include_end)?
        } else {
            List::from_values([Value::List(procedure_header_line(
                procedure,
                &mut self.interner,
            ))])
        };
        for line in lines.iter() {
            self.output.push_str(&line.show(&self.interner));
            self.output.push('\n');
        }
        Ok(())
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

    fn forever(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("forever", &args, 1)?;
        let list = list_input(&args[0], "FOREVER")?;
        loop {
            match self.execute_instruction_list(list)? {
                ControlFlow::None => {}
                ControlFlow::Stop => return Ok(PrimitiveResult::NoValue),
                control => return Ok(PrimitiveResult::Control(control)),
            }
        }
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
        let values = list_values(list_input(&args[1], "APPLY")?);
        self.invoke_template_value(&args[0], values)
            .map(PrimitiveResult::Value)
    }

    fn foreach(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("foreach", &args, 2)?;
        let values = list_values(list_input(&args[1], "FOREACH")?);
        for value in values {
            match self.invoke_template_effect(&args[0], vec![value])? {
                ControlFlow::None => {}
                control => return Ok(PrimitiveResult::Control(control)),
            }
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn map(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("map", &args, 2)?;
        let values = list_values(list_input(&args[1], "MAP")?);
        let mut mapped = Vec::new();
        for value in values {
            mapped.push(self.invoke_template_value(&args[0], vec![value])?);
        }
        Ok(PrimitiveResult::Value(Value::List(List::from_values(
            mapped,
        ))))
    }

    fn filter(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("filter", &args, 2)?;
        let values = list_values(list_input(&args[1], "FILTER")?);
        let mut kept = Vec::new();
        for value in values {
            let keep = self.invoke_template_value(&args[0], vec![value.clone()])?;
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
        for value in values {
            acc = self.invoke_template_value(&args[0], vec![acc, value])?;
        }
        Ok(PrimitiveResult::Value(acc))
    }

    fn cascade2(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("cascade.2", &args, 5)?;
        self.cascade(args)
    }

    fn transfer(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("transfer", &args, 3)?;
        let endtest = args[0].clone();
        let template = args[1].clone();
        let inbasket = list_input(&args[2], "TRANSFER")?;
        let mut outbasket = Value::List(List::empty());
        let no_endtest = matches!(&endtest, Value::List(list) if list.is_empty());

        for item in inbasket.iter() {
            let transfer_bindings = [("?in", item.clone()), ("?out", outbasket.clone())];
            if !no_endtest {
                let stop = self.invoke_template_value_with_bindings(
                    &endtest,
                    vec![item.clone(), outbasket.clone()],
                    &transfer_bindings,
                )?;
                if logo_truth(&stop, &self.interner) {
                    break;
                }
            }
            outbasket = self.invoke_template_value_with_bindings(
                &template,
                vec![item.clone(), outbasket.clone()],
                &transfer_bindings,
            )?;
        }

        Ok(PrimitiveResult::Value(outbasket))
    }

    fn cascade(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        if args.len() < 3 {
            return Err(VmError::new(format!(
                "cascade expected at least 3 input(s), got {}",
                args.len()
            )));
        }

        let endtest = args[0].clone();
        let (template_args, final_template) = if args.len().is_multiple_of(2) {
            (&args[1..args.len() - 1], Some(args[args.len() - 1].clone()))
        } else {
            (&args[1..], None)
        };
        if template_args.len() % 2 != 0 {
            return Err(VmError::new(
                "CASCADE expects template/startvalue pairs, with an optional final template",
            ));
        }

        let mut templates = Vec::new();
        let mut values = Vec::new();
        for pair in template_args.chunks_exact(2) {
            templates.push(pair[0].clone());
            values.push(pair[1].clone());
        }

        let mut rounds = 0usize;
        match &endtest {
            Value::Number(count) => {
                let count = count.get();
                if count < 0.0 || count.fract() != 0.0 {
                    return Err(VmError::new("cascade count must be a nonnegative integer"));
                }
                for round in 1..=(count as usize) {
                    values = self.cascade_round(&templates, values, round)?;
                    rounds = round;
                }
            }
            _ => loop {
                let cascade_bindings = cascade_iteration_bindings(rounds + 1);
                let stop = self.invoke_template_value_with_bindings(
                    &endtest,
                    values.clone(),
                    &cascade_bindings,
                )?;
                if logo_truth(&stop, &self.interner) {
                    break;
                }
                rounds += 1;
                values = self.cascade_round(&templates, values, rounds)?;
            },
        }

        let output = match final_template {
            Some(final_template) => {
                let cascade_bindings = cascade_iteration_bindings(rounds);
                self.invoke_template_value_with_bindings(
                    &final_template,
                    values.clone(),
                    &cascade_bindings,
                )?
            }
            None => values
                .into_iter()
                .next()
                .expect("cascade always has at least one start value"),
        };
        Ok(PrimitiveResult::Value(output))
    }

    fn cascade_round(
        &mut self,
        templates: &[Value],
        values: Vec<Value>,
        round: usize,
    ) -> Result<Vec<Value>, VmError> {
        let cascade_bindings = cascade_iteration_bindings(round);
        templates
            .iter()
            .map(|template| {
                self.invoke_template_value_with_bindings(
                    template,
                    values.clone(),
                    &cascade_bindings,
                )
            })
            .collect()
    }

    /// Classifies a template value into one of UCBLogo's three template
    /// forms: a bare procedure name, an explicit-slot list whose first
    /// member is itself a list of formal parameter names, or the implicit
    /// `?`/`?1`/`?2`-slot form (the fallback when the first member isn't a
    /// list).
    fn classify_template(&self, value: &Value) -> Result<Template, VmError> {
        match value {
            Value::Word(symbol) | Value::BareWord(symbol) => Ok(Template::Procedure(*symbol)),
            Value::List(list) => {
                if let Some(Value::List(formals)) = list.first() {
                    let params =
                        parameter_names_input(&Value::List(formals.clone()), &self.interner)?;
                    let body = list.butfirst().cloned().unwrap_or_else(List::empty);
                    Ok(Template::ExplicitSlot { params, body })
                } else {
                    Ok(Template::ImplicitSlot(list.clone()))
                }
            }
            _ => Err(VmError::new("template must be a word or a list")),
        }
    }

    fn bind_template_params(
        &mut self,
        params: &[String],
        values: Vec<Value>,
        extra_bindings: &[(&str, Value)],
    ) -> Result<(), VmError> {
        if params.len() != values.len() {
            return Err(VmError::new(format!(
                "template expected {} input(s), got {}",
                params.len(),
                values.len()
            )));
        }
        self.env.push_frame();
        for (param, value) in params.iter().zip(values) {
            self.env.define_local(param.clone(), value);
        }
        self.bind_extra_template_bindings(extra_bindings);
        Ok(())
    }

    fn bind_extra_template_bindings(&mut self, extra_bindings: &[(&str, Value)]) {
        for (name, value) in extra_bindings {
            self.env.define_local(*name, value.clone());
        }
    }

    fn bind_implicit_template_slots(&mut self, values: &[Value], extra_bindings: &[(&str, Value)]) {
        self.env.push_frame();
        for (index, value) in values.iter().cloned().enumerate() {
            let numbered = format!("?{}", index + 1);
            self.env.define_local(numbered, value.clone());
            if index == 0 {
                self.env.define_local("?", value);
            }
        }
        self.bind_extra_template_bindings(extra_bindings);
    }

    fn invoke_template_value(
        &mut self,
        template: &Value,
        values: Vec<Value>,
    ) -> Result<Value, VmError> {
        self.invoke_template_value_with_bindings(template, values, &[])
    }

    fn invoke_template_value_with_bindings(
        &mut self,
        template: &Value,
        values: Vec<Value>,
        extra_bindings: &[(&str, Value)],
    ) -> Result<Value, VmError> {
        result_value(self.invoke_template_result_with_bindings(template, values, extra_bindings)?)
    }

    fn invoke_template_effect(
        &mut self,
        template: &Value,
        values: Vec<Value>,
    ) -> Result<ControlFlow, VmError> {
        self.invoke_template_effect_with_bindings(template, values, &[])
    }

    fn invoke_template_effect_with_bindings(
        &mut self,
        template: &Value,
        values: Vec<Value>,
        extra_bindings: &[(&str, Value)],
    ) -> Result<ControlFlow, VmError> {
        match self.classify_template(template)? {
            Template::Procedure(symbol) => {
                let result = if extra_bindings.is_empty() {
                    self.call(symbol, values)
                } else {
                    self.env.push_frame();
                    self.bind_extra_template_bindings(extra_bindings);
                    let result = self.call(symbol, values);
                    self.env.pop_frame();
                    result
                }?;
                Ok(primitive_to_run_result(result).control)
            }
            Template::ExplicitSlot { params, body } => {
                self.bind_template_params(&params, values, extra_bindings)?;
                let result = self.execute_instruction_list_effect(&body);
                self.env.pop_frame();
                Ok(result?.control)
            }
            Template::ImplicitSlot(list) => {
                self.execute_template_for_effect_with_bindings(&list, values, extra_bindings)
            }
        }
    }

    fn invoke_template_result_with_bindings(
        &mut self,
        template: &Value,
        values: Vec<Value>,
        extra_bindings: &[(&str, Value)],
    ) -> Result<RunResult, VmError> {
        match self.classify_template(template)? {
            Template::Procedure(symbol) => {
                let result = if extra_bindings.is_empty() {
                    self.call(symbol, values)
                } else {
                    self.env.push_frame();
                    self.bind_extra_template_bindings(extra_bindings);
                    let result = self.call(symbol, values);
                    self.env.pop_frame();
                    result
                }?;
                Ok(primitive_to_run_result(result))
            }
            Template::ExplicitSlot { params, body } => {
                self.bind_template_params(&params, values, extra_bindings)?;
                let result = self.execute_instruction_list_result(&body);
                self.env.pop_frame();
                result
            }
            Template::ImplicitSlot(list) => {
                self.execute_template_result_with_bindings(&list, values, extra_bindings)
            }
        }
    }

    fn execute_template_for_effect_with_bindings(
        &mut self,
        template: &List,
        values: Vec<Value>,
        extra_bindings: &[(&str, Value)],
    ) -> Result<ControlFlow, VmError> {
        Ok(self
            .execute_template_effect_with_bindings(template, values, extra_bindings)?
            .control)
    }

    fn execute_template_effect_with_bindings(
        &mut self,
        template: &List,
        values: Vec<Value>,
        extra_bindings: &[(&str, Value)],
    ) -> Result<RunResult, VmError> {
        self.bind_implicit_template_slots(&values, extra_bindings);
        let result = self.execute_instruction_list_effect(template);
        self.env.pop_frame();
        result
    }

    fn execute_template_result_with_bindings(
        &mut self,
        template: &List,
        values: Vec<Value>,
        extra_bindings: &[(&str, Value)],
    ) -> Result<RunResult, VmError> {
        self.bind_implicit_template_slots(&values, extra_bindings);
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

    fn turtle_setx(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setx", &args, 1)?;
        let x = number_input(&args[0], &self.interner)?;
        let state = self.turtle.state();
        self.turtle.set_xy(x, state.position.y);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_sety(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("sety", &args, 1)?;
        let y = number_input(&args[0], &self.interner)?;
        let state = self.turtle.state();
        self.turtle.set_xy(state.position.x, y);
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

    fn init_turtle(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("init.turtle", &args, 0)?;
        self.turtle.clearscreen();
        self.turtle.show_turtle();
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_shownp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("shownp", &args, 0)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(self.turtle.state().visible),
        ))
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

/// The three UCBLogo template forms accepted by MAP/FOREACH/FILTER/REDUCE/
/// APPLY/CASCADE.
enum Template {
    Procedure(Symbol),
    ExplicitSlot { params: Vec<String>, body: List },
    ImplicitSlot(List),
}

fn primitive_to_run_result(result: PrimitiveResult) -> RunResult {
    match result {
        PrimitiveResult::Value(value) => RunResult {
            stack: vec![value],
            output: String::new(),
            control: ControlFlow::None,
        },
        PrimitiveResult::NoValue => RunResult {
            stack: Vec::new(),
            output: String::new(),
            control: ControlFlow::None,
        },
        PrimitiveResult::Control(control) => RunResult {
            stack: Vec::new(),
            output: String::new(),
            control,
        },
    }
}

fn cascade_iteration_bindings(round: usize) -> [(&'static str, Value); 2] {
    let round = Value::number(round as f64);
    [("#", round.clone()), ("repcount", round)]
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
        Value::Word(symbol) | Value::BareWord(symbol) => {
            Ok(interner.spelling(*symbol).to_string())
        }
        _ => Err(VmError::new(format!(
            "{} is not a variable name",
            value.show(interner)
        ))),
    }
}

fn property_key_input(value: &Value, interner: &Interner) -> Result<String, VmError> {
    match value {
        Value::Word(symbol) | Value::BareWord(symbol) => {
            Ok(interner.canonical_spelling(*symbol).to_string())
        }
        Value::Number(_) => Ok(value.show(interner)),
        Value::List(_) | Value::Array(_) => Err(VmError::new(format!(
            "{} is not a property-list key",
            value.show(interner)
        ))),
    }
}

fn source_text_input(value: &Value, interner: &Interner) -> String {
    match value {
        Value::Word(symbol) | Value::BareWord(symbol) => {
            interner.spelling(*symbol).to_string()
        }
        _ => value.show(interner),
    }
}

fn token_to_data_value(kind: TokenKind, interner: &mut Interner) -> Option<Value> {
    match kind {
        TokenKind::Word(word) => Some(number_or_bare_word_value(interner, word)),
        TokenKind::QuotedWord(word) => Some(Value::word(interner, word)),
        TokenKind::ColonWord(word) => Some(Value::bare_word(interner, format!(":{word}"))),
        TokenKind::Infix(op) => Some(Value::bare_word(interner, op.to_string())),
        TokenKind::LBracket => Some(Value::bare_word(interner, "[")),
        TokenKind::RBracket => Some(Value::bare_word(interner, "]")),
        TokenKind::LParen => Some(Value::bare_word(interner, "(")),
        TokenKind::RParen => Some(Value::bare_word(interner, ")")),
        TokenKind::LBrace => Some(Value::bare_word(interner, "{")),
        TokenKind::RBrace => Some(Value::bare_word(interner, "}")),
    }
}

fn number_or_bare_word_value(interner: &mut Interner, word: String) -> Value {
    match word.parse::<f64>() {
        Ok(number) if number.is_finite() => Value::number(number),
        _ => Value::bare_word(interner, word),
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
        Value::Word(symbol) | Value::BareWord(symbol) => {
            Ok(vec![interner.spelling(*symbol).to_string()])
        }
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

fn parameter_names_input(value: &Value, interner: &Interner) -> Result<Vec<String>, VmError> {
    match value {
        Value::Word(symbol) | Value::BareWord(symbol) => {
            Ok(vec![normalize_parameter_name(interner.spelling(*symbol))?])
        }
        Value::List(list) => list
            .iter()
            .map(|value| match value {
                Value::Word(symbol) | Value::BareWord(symbol) => {
                    normalize_parameter_name(interner.spelling(*symbol))
                }
                _ => Err(VmError::new(format!(
                    "{} is not a procedure input name",
                    value.show(interner)
                ))),
            })
            .collect(),
        Value::Array(array) => array
            .to_list()
            .iter()
            .map(|value| match value {
                Value::Word(symbol) | Value::BareWord(symbol) => {
                    normalize_parameter_name(interner.spelling(*symbol))
                }
                _ => Err(VmError::new(format!(
                    "{} is not a procedure input name",
                    value.show(interner)
                ))),
            })
            .collect(),
        _ => Err(VmError::new(format!(
            "{} is not a procedure input list",
            value.show(interner)
        ))),
    }
}

fn normalize_parameter_name(name: &str) -> Result<String, VmError> {
    let trimmed = name.trim_start_matches(':');
    if trimmed.is_empty() {
        Err(VmError::new("procedure input names cannot be empty"))
    } else {
        Ok(trimmed.to_string())
    }
}

fn define_body_input(
    value: &Value,
    interner: &Interner,
    arities: &ArityTable,
) -> Result<Vec<String>, VmError> {
    let list = match value {
        Value::List(list) => list,
        _ => {
            return Err(VmError::new(format!(
                "{} is not a procedure body line list",
                value.show(interner)
            )))
        }
    };
    list.iter()
        .map(|line| match line {
            Value::List(line_list) => Ok(list_to_source(line_list, interner, arities)),
            _ => Err(VmError::new(format!(
                "{} is not a procedure body line",
                line.show(interner)
            ))),
        })
        .collect()
}

fn logo_truth(value: &Value, interner: &Interner) -> bool {
    match value {
        Value::Word(symbol) | Value::BareWord(symbol) => {
            !interner.canonical_spelling(*symbol).eq("false")
        }
        Value::Number(number) => number.get() != 0.0,
        Value::List(list) => !list.is_empty(),
        Value::Array(array) => !array.is_empty(),
    }
}

fn is_error_catch_tag(value: &Value, interner: &Interner) -> bool {
    matches!(
        value,
        Value::Word(symbol) | Value::BareWord(symbol)
            if interner.canonical_spelling(*symbol).eq("error")
    )
}

fn procedure_definition_text(procedure: &Procedure, interner: &Interner) -> String {
    let mut source = String::new();
    source.push_str("to ");
    source.push_str(interner.spelling(procedure.name()));
    for param in procedure.params() {
        source.push(' ');
        source.push(':');
        source.push_str(interner.spelling(*param));
    }
    source.push('\n');
    if !procedure.body_source().is_empty() {
        source.push_str(procedure.body_source());
        if !procedure.body_source().ends_with('\n') {
            source.push('\n');
        }
    }
    source.push_str("end\n");
    source
}

fn value_source_literal(value: &Value, interner: &Interner) -> String {
    match value {
        Value::Word(symbol) => format!("\"{}", interner.spelling(*symbol)),
        Value::BareWord(symbol) => interner.spelling(*symbol).to_string(),
        _ => value.show(interner),
    }
}

fn contentslist_input(value: &Value, interner: &Interner) -> Result<EditContents, VmError> {
    if let Value::List(list) = value {
        let parts: Vec<&Value> = list.iter().collect();
        if parts.len() == 3 && parts.iter().all(|part| matches!(part, Value::List(_))) {
            return Ok(EditContents {
                procedures: local_names(parts[0], interner)?,
                variables: local_names(parts[1], interner)?,
                plists: local_names(parts[2], interner)?,
            });
        }
    }
    Ok(EditContents {
        procedures: local_names(value, interner)?,
        variables: Vec::new(),
        plists: Vec::new(),
    })
}

fn resolve_editor_command(
    override_command: Option<&str>,
    env_lookup: Result<String, env::VarError>,
) -> Result<String, VmError> {
    if let Some(command) = override_command {
        return Ok(command.to_string());
    }
    env_lookup.map_err(|_| VmError::new("EDIT requires the EDITOR environment variable to be set"))
}

fn edit_temp_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "dynalogo-edit-{}-{unique}.logo",
        std::process::id()
    ))
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
    let mut values = vec![
        Value::bare_word(interner, "to"),
        Value::BareWord(procedure.name()),
    ];
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

fn primitive_names() -> &'static [&'static str] {
    &[
        "sum",
        "+",
        "difference",
        "-",
        "product",
        "*",
        "quotient",
        "/",
        "remainder",
        "abs",
        "int",
        "round",
        "sqrt",
        "sin",
        "cos",
        "tan",
        "random",
        "rerandom",
        "and",
        "or",
        "not",
        "equalp",
        "equal?",
        "emptyp",
        "empty?",
        "memberp",
        "member?",
        "first",
        "butfirst",
        "bf",
        "last",
        "butlast",
        "bl",
        "fput",
        "lput",
        "sentence",
        "se",
        "list",
        "word",
        "count",
        "item",
        "wordp",
        "listp",
        "numberp",
        "intp",
        "decimalp",
        "print",
        "pr",
        "show",
        "type",
        "load",
        "save",
        "setread",
        "setwrite",
        "readchar",
        "rc",
        "readlist",
        "rl",
        "readword",
        "rw",
        "openread",
        "openwrite",
        "openappend",
        "close",
        "reader",
        "writer",
        "dribble",
        "nodribble",
        "make",
        "name",
        "thing",
        "local",
        "namep",
        "definedp",
        "defined?",
        "primitivep",
        "primitive?",
        "text",
        "fulltext",
        "copydef",
        "edit",
        "ed",
        "po",
        "poall",
        "pons",
        "pops",
        "pots",
        ".primitives",
        "erase",
        "er",
        "ern",
        "erns",
        "erps",
        "erall",
        "pprop",
        "gprop",
        "remprop",
        "plist",
        "array",
        "setitem",
        "listtoarray",
        "arraytolist",
        "repeat",
        "if",
        "ifelse",
        "run",
        "runresult",
        "parse",
        "runparse",
        "apply",
        "foreach",
        "map",
        "filter",
        "reduce",
        "cascade",
        "cascade.2",
        "transfer",
        "repcount",
        "test",
        "iftrue",
        "ift",
        "iffalse",
        "iff",
        "wait",
        "catch",
        "throw",
        "error",
        "pause",
        "continue",
        "forward",
        "fd",
        "back",
        "bk",
        "left",
        "lt",
        "right",
        "rt",
        "setxy",
        "setpos",
        "setheading",
        "seth",
        "home",
        "clearscreen",
        "cs",
        "penup",
        "pu",
        "pendown",
        "pd",
        "setpencolor",
        "setpc",
        "setpensize",
        "hideturtle",
        "ht",
        "showturtle",
        "st",
        "pos",
        "heading",
        "xcor",
        "ycor",
        "output",
        "op",
        "stop",
    ]
}

fn is_primitive_name(name: &str) -> bool {
    primitive_names().contains(&name.to_ascii_lowercase().as_str())
}

fn is_protected_workspace_procedure(name: &str) -> bool {
    matches!(
        name,
        "__whileloop"
            | "while"
            | "__untilloop"
            | "until"
            | "do.while"
            | "__condrest"
            | "cond"
            | "__caserest"
            | "case"
            | "__forloop"
            | "for"
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
    let values = list.iter().collect::<Vec<_>>();
    let mut rendered = Vec::new();
    let mut index = 0;
    while index < values.len() {
        let (fragment, consumed) = value_expr_to_source(&values[index..], interner, arities);
        rendered.push(fragment);
        index += consumed.max(1);
    }
    rendered.join(" ")
}

fn value_expr_to_source(
    values: &[&Value],
    interner: &Interner,
    arities: &ArityTable,
) -> (String, usize) {
    let Some(first) = values.first() else {
        return (String::new(), 0);
    };

    match first {
        Value::List(inner) => (format!("[{}]", list_to_source(inner, interner, arities)), 1),
        Value::Word(symbol) => (format!("\"{}", interner.spelling(*symbol)), 1),
        Value::BareWord(symbol) => {
            let spelling = interner.spelling(*symbol);
            if let Some(binding) = template_binding_name(spelling) {
                return (format!(":{binding}"), 1);
            }
            if matches!(spelling, "(" | ")" | "[" | "]" | "{" | "}")
                || spelling.starts_with(':')
                || is_operator_word(spelling)
            {
                return (spelling.to_string(), 1);
            }
            if let Some(Arity::Exact(argc)) = arities.get(spelling) {
                let mut consumed = 1;
                let mut parts = vec![spelling.to_string()];
                for _ in 0..argc {
                    if consumed >= values.len() {
                        break;
                    }
                    let (arg, arg_consumed) = value_expr_to_source(&values[consumed..], interner, arities);
                    parts.push(arg);
                    consumed += arg_consumed.max(1);
                }
                return (parts.join(" "), consumed);
            }
            (format!("\"{spelling}"), 1)
        }
        Value::Number(_) | Value::Array(_) => (first.show(interner), 1),
    }
}

fn before_text(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return a.is_empty();
    }

    let mut left = a.chars();
    let mut right = b.chars();
    loop {
        match (left.next(), right.next()) {
            (None, None) => return false,
            (None, Some(_)) => return true,
            (Some(_), None) => return false,
            (Some(l), Some(r)) if l != r => return l < r,
            (Some(_), Some(_)) => {}
        }
    }
}

fn insert_sorted_tree(value: Value, tree: &List, interner: &Interner) -> Result<List, VmError> {
    if tree.is_empty() {
        return Ok(List::from_values([
            Value::List(List::empty()),
            value,
            Value::List(List::empty()),
        ]));
    }
    if tree.len() != 3 {
        return Err(VmError::new("sort tree must have three elements"));
    }

    let left = match tree.item(1) {
        Some(Value::List(list)) => list.clone(),
        _ => return Err(VmError::new("sort tree left branch must be a list")),
    };
    let current = tree
        .item(2)
        .cloned()
        .ok_or_else(|| VmError::new("sort tree is missing a value"))?;
    let right = match tree.item(3) {
        Some(Value::List(list)) => list.clone(),
        _ => return Err(VmError::new("sort tree right branch must be a list")),
    };

    if before_text(&value.show(interner), &current.show(interner)) {
        Ok(List::from_values([
            Value::List(insert_sorted_tree(value, &left, interner)?),
            current,
            Value::List(right),
        ]))
    } else {
        Ok(List::from_values([
            Value::List(left),
            current,
            Value::List(insert_sorted_tree(value, &right, interner)?),
        ]))
    }
}

fn flatten_sorted_tree(tree: &List) -> Result<List, VmError> {
    if tree.is_empty() {
        return Ok(List::empty());
    }
    if tree.len() != 3 {
        return Err(VmError::new("sort tree must have three elements"));
    }
    let left = match tree.item(1) {
        Some(Value::List(list)) => flatten_sorted_tree(list)?,
        _ => return Err(VmError::new("sort tree left branch must be a list")),
    };
    let current = tree
        .item(2)
        .cloned()
        .ok_or_else(|| VmError::new("sort tree is missing a value"))?;
    let right = match tree.item(3) {
        Some(Value::List(list)) => flatten_sorted_tree(list)?,
        _ => return Err(VmError::new("sort tree right branch must be a list")),
    };

    Ok(List::from_values(
        left.iter()
            .cloned()
            .chain(std::iter::once(current))
            .chain(right.iter().cloned()),
    ))
}

fn is_operator_word(text: &str) -> bool {
    matches!(
        text,
        "+" | "-" | "*" | "/" | "=" | "<" | ">" | "<=" | ">=" | "<>"
    )
}

fn template_binding_name(text: &str) -> Option<String> {
    if text == "?" || text == "#" {
        return Some(text.to_string());
    }

    let rest = text.strip_prefix('?')?;
    if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
        return Some(text.to_string());
    }
    if rest.eq_ignore_ascii_case("in") {
        return Some("?in".to_string());
    }
    if rest.eq_ignore_ascii_case("out") {
        return Some("?out".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turtle::{Point, TurtleEvent};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn run(source: &str) -> Result<(RunResult, Vm), VmError> {
        let mut vm = Vm::new();
        let result = vm.eval_source(source)?;
        Ok((result, vm))
    }

    fn temp_test_path(stem: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "dynalogo-{stem}-{}-{unique}.logo",
            std::process::id()
        ))
    }

    fn set_path_var(vm: &mut Vm, name: &str, path: &Path) {
        let value = Value::word(vm.interner_mut(), path.to_string_lossy());
        vm.env_mut().set_global(name, value);
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
             print which \"b [a b c] \
             print which \"z [a b c] \
             print before \"bar \"baz \
             print supersort sort [a d e f t c z] [] \
             print emptyp [] \
             print emptyp \" \
             print memberp \"b [a b c] \
             print memberp \"x [a b c]")
        .unwrap();
        assert_eq!(
            result.output,
            "3\nb\n2\n0\ntrue\n[a c d e f t z]\ntrue\ntrue\ntrue\nfalse\n"
        );
    }

    #[test]
    fn dot_draws_at_target_without_moving_the_turtle() {
        let (_, vm) = run("setxy 10 20 dot [30 40] print pos").unwrap();
        assert_eq!(vm.turtle().state().position, Point::new(10.0, 20.0));
        let line = vm
            .turtle()
            .backend()
            .events()
            .iter()
            .find_map(|event| match event {
                TurtleEvent::Line { from, to, .. }
                    if *from == Point::new(30.0, 40.0) && *to == Point::new(30.0, 40.0) =>
                {
                    Some((*from, *to))
                }
                _ => None,
            });
        assert!(line.is_some());
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
    fn full_template_forms_and_cascade_are_available() {
        let (result, _) = run("print map [[x] product :x :x] [1 2 3] \
             print apply [[a b] sum :a :b] [3 4] \
             print reduce [[acc item] sum :acc :item] [1 2 3 4] \
             print cascade 5 [product ? repcount] 1 \
             print cascade [? > 100] [sum ? ?] 1 \
             print (cascade 3 [sum ? 1] 10 [product ? 2])")
        .unwrap();
        assert_eq!(result.output, "[1 4 9]\n7\n10\n120\n128\n26\n");
    }

    #[test]
    fn cascade2_and_transfer_follow_ucblogo_semantics() {
        let (result, _) = run("print cascade 5 [lput # ?] [] \
             print cascade.2 5 [sum ?1 ?2] 1 [?1] 0 \
             print transfer [] [lput ?in ?out] [a b c] \
             print transfer [equalp ?in \"halt] [lput ?in ?out] [a b halt c]")
        .unwrap();
        assert_eq!(result.output, "[1 2 3 4 5]\n8\n[a b c]\n[a b]\n");
    }

    #[test]
    fn instruction_list_reserialization_preserves_literal_words_that_shadow_primitives() {
        let (result, _) = run("make \"?in \"go print runresult [equalp ?in \"stop]").unwrap();
        assert_eq!(result.output, "false\n");
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
    fn forever_repeats_until_body_stops() {
        let (result, _) =
            run("make \"x 0 forever [make \"x sum :x 1 if :x = 3 [stop]] print :x").unwrap();
        assert_eq!(result.output, "3\n");
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
             make \"foo 99
             print definedp \"square
             print definedp \"sum
             print primitivep \"sum
             print primitivep \"square
             print primitive? \"fd
             print namep \"foo
             print wordp \"hello
             print listp [a b]
             print numberp \"123
             print intp 5
             print decimalp 5.5")
        .unwrap();
        assert_eq!(
            result.output,
            "true\nfalse\ntrue\nfalse\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\n"
        );
    }

    #[test]
    fn atari_math_primitives_cover_core_numeric_helpers() {
        let (result, _) = run("print abs -3
             print int 4.9
             print round 4.6
             print sqrt 9
             print sin 30
             print cos 60
             rerandom
             print random 10
             rerandom
             print random 10")
        .unwrap();
        let lines = result.output.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "3");
        assert_eq!(lines[1], "4");
        assert_eq!(lines[2], "5");
        assert_eq!(lines[3], "3");
        assert_eq!(lines[4], "0.49999999999999994");
        assert_eq!(lines[5], "0.5000000000000001");
        assert_eq!(lines[6], lines[7]);
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
    fn workspace_listing_commands_report_user_procedures_and_variables() {
        let mut vm = Vm::new();
        vm.eval_source(
            "to alpha :x
             print :x
             end
             to beta :y
             output sum :y 1
             end
             make \"foo 7
             make \"bar [a b]",
        )
        .unwrap();
        let result = vm
            .eval_source("pots pops pons poall po [alpha] .primitives")
            .unwrap();
        assert!(result.output.contains("[to alpha :x]"));
        assert!(result.output.contains("[to beta :y]"));
        assert!(result.output.contains("[print :x]"));
        assert!(result.output.contains("[output sum :y 1]"));
        assert!(result.output.contains("bar [a b]"));
        assert!(result.output.contains("foo 7"));
        assert!(result.output.contains("sum + difference"));
        assert!(!result.output.contains("__whileloop"));
    }

    #[test]
    fn workspace_erase_commands_remove_user_bindings() {
        let mut vm = Vm::new();
        vm.eval_source(
            "to alpha :x
             output :x
             end
             to beta :y
             output :y
             end
             make \"foo 7
             make \"bar 8",
        )
        .unwrap();
        vm.eval_source("ern [foo] erase [alpha]").unwrap();
        let result = vm
            .eval_source("print namep \"foo print definedp \"alpha print definedp \"beta")
            .unwrap();
        assert_eq!(result.output, "false\nfalse\ntrue\n");

        vm.clear_output();
        vm.eval_source("erns erps").unwrap();
        vm.clear_output();
        let result = vm
            .eval_source("print namep \"bar print definedp \"beta print definedp \"while")
            .unwrap();
        assert_eq!(result.output, "false\nfalse\ntrue\n");
    }

    #[test]
    fn copydef_clones_a_workspace_procedure() {
        let mut vm = Vm::new();
        vm.eval_source(
            "to square :x
             output product :x :x
             end
             copydef \"quad \"square",
        )
        .unwrap();
        let result = vm.eval_source("print quad 6 print text \"quad").unwrap();
        assert_eq!(result.output, "36\n[[to quad :x] [output product :x :x]]\n");
        assert!(vm.procedures().contains_key("quad"));
    }

    #[test]
    fn define_builds_a_workspace_procedure_from_logo_data() {
        let mut vm = Vm::new();
        vm.eval_source("define \"square [size] [[output product :size :size]]")
            .unwrap();
        let result = vm
            .eval_source("print square 5 print text \"square")
            .unwrap();
        assert_eq!(
            result.output,
            "25\n[[to square :size] [output product :size :size]]\n"
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
    fn file_stream_primitives_load_and_save_workspace() {
        let load_path = temp_test_path("load-source");
        let save_path = temp_test_path("save-workspace");
        fs::write(
            &load_path,
            "to triple :x\noutput product :x 3\nend\nmake \"fromfile 17\n",
        )
        .unwrap();

        let mut vm = Vm::new();
        set_path_var(&mut vm, "loadpath", &load_path);
        set_path_var(&mut vm, "savepath", &save_path);

        vm.eval_source("load :loadpath").unwrap();
        let loaded = vm.eval_source("print triple 4 print :fromfile").unwrap();
        assert_eq!(loaded.output, "12\n17\n");

        vm.clear_output();
        vm.eval_source(
            "to square :x
             output product :x :x
             end
             save :savepath",
        )
        .unwrap();
        let saved = fs::read_to_string(&save_path).unwrap();
        assert!(saved.contains("to square :x"));
        assert!(saved.contains("output product :x :x"));
        assert!(saved.contains("to triple :x"));
        assert!(saved.contains("output product :x 3"));
        assert!(!saved.contains("__whileloop"));
        assert!(!saved.contains("fromfile"));

        let _ = fs::remove_file(load_path);
        let _ = fs::remove_file(save_path);
    }

    #[cfg(unix)]
    fn write_fake_editor(capture_path: &std::path::Path, append: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let script_path = temp_test_path("fake-editor");
        let script = format!(
            "#!/bin/sh\ncp \"$1\" \"{}\"\ncat >> \"$1\" <<'EOF'\n{append}\nEOF\n",
            capture_path.display(),
        );
        fs::write(&script_path, script).unwrap();
        let mut perms = fs::metadata(&script_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).unwrap();
        script_path
    }

    #[cfg(unix)]
    #[test]
    fn edit_renders_procedure_and_reloads_appended_definition() {
        let capture_path = temp_test_path("edit-capture-proc");
        let editor_path = write_fake_editor(&capture_path, "to added\noutput 42\nend");

        let mut vm = Vm::new();
        vm.set_editor_command(editor_path.to_string_lossy().to_string());
        vm.eval_source("to greet\nprint \"hi\nend").unwrap();

        vm.eval_source("edit \"greet").unwrap();

        let captured = fs::read_to_string(&capture_path).unwrap();
        assert!(captured.contains("to greet"));
        assert!(captured.contains("print \"hi"));

        let result = vm.eval_source("greet print added").unwrap();
        assert_eq!(result.output, "hi\n42\n");

        let _ = fs::remove_file(capture_path);
        let _ = fs::remove_file(editor_path);
    }

    #[cfg(unix)]
    #[test]
    fn edit_contentslist_form_renders_variable_and_plist_and_reloads_changes() {
        let capture_path = temp_test_path("edit-capture-contents");
        let editor_path = write_fake_editor(&capture_path, "pprop \"colors \"blue 2");

        let mut vm = Vm::new();
        vm.set_editor_command(editor_path.to_string_lossy().to_string());
        vm.eval_source("make \"count 5 pprop \"colors \"red 1")
            .unwrap();

        vm.eval_source("edit [[] [count] [colors]]").unwrap();

        let captured = fs::read_to_string(&capture_path).unwrap();
        assert!(captured.contains("make \"count 5"));
        assert!(captured.contains("pprop \"colors \"red 1"));

        let result = vm
            .eval_source("print gprop \"colors \"blue print gprop \"colors \"red print :count")
            .unwrap();
        assert_eq!(result.output, "2\n1\n5\n");

        let _ = fs::remove_file(capture_path);
        let _ = fs::remove_file(editor_path);
    }

    #[cfg(unix)]
    #[test]
    fn edit_with_no_args_reuses_previous_edit_buffer() {
        let capture_path = temp_test_path("edit-capture-noargs");
        let first_editor = write_fake_editor(&capture_path, "make \"first 1");

        let mut vm = Vm::new();
        vm.set_editor_command(first_editor.to_string_lossy().to_string());
        vm.eval_source("edit \"nonexistent").unwrap();

        let after_first = vm.eval_source("print :first").unwrap();
        assert_eq!(after_first.output, "1\n");
        vm.clear_output();

        let noop_editor = write_fake_editor(&capture_path, "");
        vm.set_editor_command(noop_editor.to_string_lossy().to_string());
        vm.eval_source("(edit)").unwrap();

        let after_second = vm.eval_source("print :first").unwrap();
        assert_eq!(after_second.output, "1\n");

        let _ = fs::remove_file(capture_path);
        let _ = fs::remove_file(first_editor);
        let _ = fs::remove_file(noop_editor);
    }

    #[test]
    fn edit_rejects_more_than_one_input() {
        let mut vm = Vm::new();
        let error = vm.eval_source("(edit \"a \"b)").unwrap_err();
        assert!(error.message.contains("edit expected 0 or 1 input"));
    }

    #[test]
    fn edit_with_no_args_and_no_prior_session_is_an_error() {
        let mut vm = Vm::new();
        let error = vm.eval_source("(edit)").unwrap_err();
        assert!(error.message.contains("EDIT needs a contents list"));
    }

    #[test]
    fn resolve_editor_command_prefers_override_then_env_then_errors() {
        assert_eq!(
            resolve_editor_command(Some("nano"), Err(env::VarError::NotPresent)).unwrap(),
            "nano"
        );
        assert_eq!(
            resolve_editor_command(None, Ok("vim".to_string())).unwrap(),
            "vim"
        );
        let error = resolve_editor_command(None, Err(env::VarError::NotPresent)).unwrap_err();
        assert!(error.message.contains("EDITOR environment variable"));
    }

    #[test]
    fn file_stream_primitives_setread_readchar_and_readlist() {
        let read_path = temp_test_path("read-stream");
        fs::write(&read_path, "hello\nsum 1 2\n").unwrap();

        let mut vm = Vm::new();
        set_path_var(&mut vm, "readpath", &read_path);
        let result = vm
            .eval_source("setread :readpath print rc print rc setread :readpath print rl print rl")
            .unwrap();
        assert_eq!(result.output, "h\ne\n[hello]\n[sum 1 2]\n");

        let _ = fs::remove_file(read_path);
    }

    #[test]
    fn file_stream_primitives_setwrite_redirects_output() {
        let write_path = temp_test_path("write-stream");

        let mut vm = Vm::new();
        set_path_var(&mut vm, "writepath", &write_path);
        let result = vm
            .eval_source(
                "setwrite :writepath type \"hi print [a b] show 42 setwrite [] print \"done",
            )
            .unwrap();
        assert_eq!(result.output, "done\n");
        assert_eq!(fs::read_to_string(&write_path).unwrap(), "hi[a b]\n42\n");

        let _ = fs::remove_file(write_path);
    }

    #[test]
    fn readword_reads_raw_line_as_single_word() {
        let read_path = temp_test_path("readword-stream");
        fs::write(&read_path, "hello there\nsecond line\n").unwrap();

        let mut vm = Vm::new();
        set_path_var(&mut vm, "readpath", &read_path);
        let result = vm
            .eval_source("setread :readpath print readword print readword")
            .unwrap();
        assert_eq!(result.output, "hello there\nsecond line\n");

        let _ = fs::remove_file(read_path);
    }

    #[test]
    fn openread_setread_switching_preserves_position_and_close_resets_reader() {
        let path_a = temp_test_path("stream-a");
        let path_b = temp_test_path("stream-b");
        fs::write(&path_a, "aa\nbb\n").unwrap();
        fs::write(&path_b, "cc\ndd\n").unwrap();

        let mut vm = Vm::new();
        set_path_var(&mut vm, "a", &path_a);
        set_path_var(&mut vm, "b", &path_b);

        let result = vm
            .eval_source(
                "openread :a openread :b \
                 setread :a print rc \
                 setread :b print rc \
                 setread :a print rc \
                 close :a print reader \
                 close :b",
            )
            .unwrap();
        assert_eq!(result.output, "a\nc\na\n[]\n");

        let _ = fs::remove_file(path_a);
        let _ = fs::remove_file(path_b);
    }

    #[test]
    fn setread_without_openread_always_reopens_from_the_start() {
        let read_path = temp_test_path("bare-setread-stream");
        fs::write(&read_path, "hello\nsum 1 2\n").unwrap();

        let mut vm = Vm::new();
        set_path_var(&mut vm, "readpath", &read_path);
        let result = vm
            .eval_source("setread :readpath print rc print rc setread :readpath print rl print rl")
            .unwrap();
        assert_eq!(result.output, "h\ne\n[hello]\n[sum 1 2]\n");

        let _ = fs::remove_file(read_path);
    }

    #[test]
    fn close_errors_for_unopened_stream() {
        let mut vm = Vm::new();
        let error = vm.eval_source("close \"nope").unwrap_err();
        assert!(error.message.contains("is not open"));
    }

    #[test]
    fn openwrite_truncates_and_openappend_preserves_existing_content() {
        let write_path = temp_test_path("openwrite-stream");
        fs::write(&write_path, "existing\n").unwrap();

        let mut vm = Vm::new();
        set_path_var(&mut vm, "writepath", &write_path);

        vm.eval_source("openappend :writepath setwrite :writepath type \"more setwrite []")
            .unwrap();
        assert_eq!(fs::read_to_string(&write_path).unwrap(), "existing\nmore");

        vm.eval_source("openwrite :writepath setwrite :writepath type \"fresh setwrite []")
            .unwrap();
        assert_eq!(fs::read_to_string(&write_path).unwrap(), "fresh");

        let _ = fs::remove_file(write_path);
    }

    #[test]
    fn reader_and_writer_report_current_stream_names() {
        let read_path = temp_test_path("reader-stream");
        let write_path = temp_test_path("writer-stream");
        fs::write(&read_path, "x\n").unwrap();

        let mut vm = Vm::new();
        set_path_var(&mut vm, "readpath", &read_path);
        set_path_var(&mut vm, "writepath", &write_path);

        let before = vm.eval_source("print reader print writer").unwrap();
        assert_eq!(before.output, "[]\n[]\n");

        vm.clear_output();
        let during_read = vm.eval_source("setread :readpath print reader").unwrap();
        assert_eq!(
            during_read.output,
            format!("{}\n", read_path.to_string_lossy())
        );

        vm.eval_source("setwrite :writepath print writer setwrite []")
            .unwrap();
        assert_eq!(
            fs::read_to_string(&write_path).unwrap(),
            format!("{}\n", write_path.to_string_lossy())
        );

        vm.clear_output();
        let after = vm
            .eval_source("setread [] print reader print writer")
            .unwrap();
        assert_eq!(after.output, "[]\n[]\n");

        let _ = fs::remove_file(read_path);
        let _ = fs::remove_file(write_path);
    }

    #[test]
    fn dribble_records_screen_output_but_not_redirected_writes() {
        let dribble_path = temp_test_path("dribble-log");
        let write_path = temp_test_path("dribble-write-target");

        let mut vm = Vm::new();
        set_path_var(&mut vm, "dribblepath", &dribble_path);
        set_path_var(&mut vm, "writepath", &write_path);

        let result = vm
            .eval_source(
                "dribble :dribblepath print \"visible \
                 setwrite :writepath print \"hidden setwrite [] \
                 print \"back nodribble print \"after",
            )
            .unwrap();
        assert_eq!(result.output, "visible\nback\nafter\n");
        assert_eq!(
            fs::read_to_string(&dribble_path).unwrap(),
            "visible\nback\n"
        );
        assert_eq!(fs::read_to_string(&write_path).unwrap(), "hidden\n");

        let _ = fs::remove_file(dribble_path);
        let _ = fs::remove_file(write_path);
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

    #[test]
    fn atari_turtle_state_primitives_setx_sety_and_shownp() {
        let (result, vm) =
            run("ht print shownp setx 25 sety -10 st print shownp print pos").unwrap();
        assert_eq!(result.output, "false\ntrue\n[25 -10]\n");
        assert_eq!(vm.turtle().state().position, Point::new(25.0, -10.0));
        assert!(vm.turtle().state().visible);
    }

    #[test]
    fn init_turtle_resets_display_to_visible_default_turtle() {
        let (_, vm) = run("ht setxy 10 20 init.turtle").unwrap();
        let state = vm.turtle().state();
        assert_eq!(state.position, Point::new(0.0, 0.0));
        assert_eq!(state.heading, 0.0);
        assert!(state.visible);
        assert!(vm
            .turtle()
            .backend()
            .events()
            .iter()
            .any(|event| matches!(event, TurtleEvent::Clear)));
    }
}
