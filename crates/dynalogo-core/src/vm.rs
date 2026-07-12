//! Stack VM for DynaLOGO bytecode.
//!
//! This is the initial v0.1 executor. It establishes the pieces that later
//! procedure support will build on: a shared interner, dynamic-scope frame
//! stack, primitive dispatch, bytecode stack execution, and `OUTPUT`/`STOP`
//! control signals.

use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::f64::consts::PI;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::bytecode::{
    Chunk, ChunkCache, ChunkKey, CompileMode, Compiler, Instruction, OutputTarget,
};
use crate::collision::{self, detect_collisions, CollisionConfig};
use crate::demon::{DemonCondition, DemonEvent, DemonScheduler};
use crate::dynaturtle::{TurtleId, TurtleStore};
use crate::lexer::{lex, InfixOp, TokenKind};
use crate::parser::{parse_source, Arity, ArityTable, ParseError};
use crate::turtle::{Point, TurtleEvent};
use crate::value::{Interner, List, LogoArray, LogoNumber, Symbol, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlow {
    None,
    Output(Value),
    Stop,
    Continue,
    Throw { tag: Value, value: Value },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    pub stack: Vec<Value>,
    pub output: String,
    pub control: ControlFlow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ErrorInfo {
    code: i32,
    message: String,
    procedure: String,
    instruction_line: String,
}

impl ErrorInfo {
    fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            procedure: String::new(),
            instruction_line: String::new(),
        }
    }

    fn to_value(&self, interner: &mut Interner) -> Value {
        Value::list([
            Value::number(f64::from(self.code)),
            Value::word(interner, &self.message),
            Value::word(interner, &self.procedure),
            Value::word(interner, &self.instruction_line),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmError {
    pub message: String,
    info: Option<ErrorInfo>,
}

impl VmError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            info: None,
        }
    }

    fn with_info(message: impl Into<String>, info: ErrorInfo) -> Self {
        Self {
            message: message.into(),
            info: Some(info),
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

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextScreenMode {
    SplitScreen,
    FullScreen,
    TextScreen,
}

impl TextScreenMode {
    fn name(self) -> &'static str {
        match self {
            Self::SplitScreen => "splitscreen",
            Self::FullScreen => "fullscreen",
            Self::TextScreen => "textscreen",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutsideWorldState {
    key_buffer: Vec<char>,
    joystick_directions: [u8; 4],
    joystick_buttons: [bool; 4],
    paddle_positions: [u8; 8],
    paddle_buttons: [bool; 8],
    sound_envelope: [u8; 4],
    text_cursor: (usize, usize),
    timeout_sixtieths: usize,
    text_screen_mode: TextScreenMode,
}

impl Default for OutsideWorldState {
    fn default() -> Self {
        Self {
            key_buffer: Vec::new(),
            joystick_directions: [15; 4],
            joystick_buttons: [false; 4],
            paddle_positions: [0; 8],
            paddle_buttons: [false; 8],
            sound_envelope: [0; 4],
            text_cursor: (0, 0),
            timeout_sixtieths: 0,
            text_screen_mode: TextScreenMode::SplitScreen,
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
    buried_names: HashSet<String>,
    turtles: TurtleStore,
    demons: DemonScheduler,
    collision_config: CollisionConfig,
    demon_fuel: usize,
    read_stream: Option<InputStream>,
    current_read_managed: bool,
    read_streams: HashMap<String, InputStream>,
    current_write: Option<String>,
    write_streams: HashSet<String>,
    dribble: Option<PathBuf>,
    outside_world: OutsideWorldState,
    test_result: Option<bool>,
    caught_error: Option<ErrorInfo>,
    call_stack: Vec<String>,
    pause_depth: usize,
    pause_inputs: VecDeque<String>,
    random_seed: u64,
    last_toot: Option<[u8; 4]>,
    edit_buffer: Option<EditSession>,
    editor_override: Option<String>,
    chunk_cache: ChunkCache,
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
    is_macro: bool,
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

    pub fn is_macro(&self) -> bool {
        self.is_macro
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
            buried_names: HashSet::new(),
            turtles: TurtleStore::new(),
            demons: DemonScheduler::new(),
            collision_config: CollisionConfig::default(),
            demon_fuel: 256,
            read_stream: None,
            current_read_managed: false,
            read_streams: HashMap::new(),
            current_write: None,
            write_streams: HashSet::new(),
            dribble: None,
            outside_world: OutsideWorldState::default(),
            test_result: None,
            caught_error: None,
            call_stack: Vec::new(),
            pause_depth: 0,
            pause_inputs: VecDeque::new(),
            random_seed: 0x4d595df4d0f33173,
            last_toot: None,
            edit_buffer: None,
            editor_override: None,
            chunk_cache: ChunkCache::new(),
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
    pub fn set_editor_command(&mut self, command: impl Into<String>) {
        self.editor_override = Some(command.into());
    }

    pub fn output(&self) -> &str {
        &self.output
    }

    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    pub fn push_keypress(&mut self, key: char) {
        self.outside_world.key_buffer.push(key);
    }

    pub fn clear_keypresses(&mut self) {
        self.outside_world.key_buffer.clear();
    }

    pub fn set_joystick_state(&mut self, index: usize, direction: u8, button: bool) {
        if index < self.outside_world.joystick_directions.len() {
            self.outside_world.joystick_directions[index] = direction;
            self.outside_world.joystick_buttons[index] = button;
        }
    }

    pub fn set_paddle_state(&mut self, index: usize, position: u8, button: bool) {
        if index < self.outside_world.paddle_positions.len() {
            self.outside_world.paddle_positions[index] = position;
            self.outside_world.paddle_buttons[index] = button;
        }
    }

    pub fn sound_envelope(&self) -> [u8; 4] {
        self.outside_world.sound_envelope
    }

    pub fn text_cursor(&self) -> (usize, usize) {
        self.outside_world.text_cursor
    }

    pub fn timeout_sixtieths(&self) -> usize {
        self.outside_world.timeout_sixtieths
    }

    pub fn text_screen_mode(&self) -> &'static str {
        self.outside_world.text_screen_mode.name()
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
        error
    }

    fn current_error_context(&self) -> Option<&str> {
        self.call_stack
            .iter()
            .rev()
            .find(|name| name.as_str() != "catch")
            .map(String::as_str)
    }

    fn contextualize_error(&self, mut error: VmError) -> VmError {
        let mut info = error
            .info
            .take()
            .or_else(|| infer_error_info(&error.message));
        if let Some(ref mut info) = info {
            if info.procedure.is_empty() {
                if let Some(name) = self.current_error_context() {
                    info.procedure = name.to_string();
                }
            }
        }
        error.info = info;
        error
    }

    fn error_with_code(&self, code: i32, message: impl Into<String>) -> VmError {
        let mut info = ErrorInfo::new(code, message.into());
        if let Some(name) = self.current_error_context() {
            info.procedure = name.to_string();
        }
        VmError::with_info(info.message.clone(), info)
    }

    fn record_caught_error(&mut self, error: &VmError) {
        self.caught_error = error
            .info
            .clone()
            .or_else(|| infer_error_info(&error.message))
            .map(|mut info| {
                if info.procedure.is_empty() {
                    if let Some(name) = self.current_error_context() {
                        info.procedure = name.to_string();
                    }
                }
                info
            });
    }

    pub fn procedures(&self) -> &HashMap<String, Procedure> {
        &self.procedures
    }

    pub fn property_lists(&self) -> &HashMap<String, HashMap<String, Value>> {
        &self.property_lists
    }

    pub fn turtles(&self) -> &TurtleStore {
        &self.turtles
    }

    pub fn turtles_mut(&mut self) -> &mut TurtleStore {
        &mut self.turtles
    }

    pub fn last_toot(&self) -> Option<[u8; 4]> {
        self.last_toot
    }

    pub fn demons(&self) -> &DemonScheduler {
        &self.demons
    }

    pub fn set_collision_config(&mut self, config: CollisionConfig) {
        self.collision_config = config;
    }

    pub fn set_demon_fuel(&mut self, fuel: usize) {
        self.demon_fuel = fuel;
    }

    pub fn push_pause_input(&mut self, line: impl Into<String>) {
        self.pause_inputs.push_back(line.into());
    }

    /// Advances the dynaturtle simulation by one fixed tick: integrates
    /// velocities, runs collision detection, feeds the report into the demon
    /// scheduler, and executes the bodies of demons that fire (fuel-limited so
    /// a burst of events cannot stall a tick).
    pub fn dynaturtle_tick(&mut self, dt_seconds: f64) -> Result<ControlFlow, VmError> {
        self.turtles.integrate(dt_seconds);
        let report = detect_collisions(&self.turtles, self.collision_config);
        self.demons
            .push_collision_report(report.turtle_pairs, report.edge_contacts);
        let mut over_colors = Vec::new();
        for demon in self.demons.demons() {
            if let DemonCondition::OverColor(color) = demon.condition() {
                if !over_colors.contains(color) {
                    over_colors.push(*color);
                }
            }
        }
        for color in over_colors {
            for index in 0..self.turtles.len() {
                let id = TurtleId::new(index);
                if turtle_over_color(self.turtles.events(), &self.turtles, id, color) {
                    self.demons
                        .push_event(DemonEvent::OverColor { turtle: id, color });
                }
            }
        }
        let drained = self.demons.drain_with_fuel(self.demon_fuel).drained;
        for item in drained {
            let ids = demon_event_turtles(&item.event);
            if !ids.is_empty() {
                self.turtles.tell_many(ids);
            }
            match self.execute_instruction_list(&item.body)? {
                ControlFlow::None => {}
                control => return Ok(control),
            }
        }
        Ok(ControlFlow::None)
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
                .map_err(vm_error_from_parse)?;
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
        self.define_procedure_impl(name, params, body, false)
    }

    pub fn define_macro(
        &mut self,
        name: impl AsRef<str>,
        params: Vec<String>,
        body: &str,
    ) -> Result<(), VmError> {
        self.define_procedure_impl(name, params, body, true)
    }

    fn define_procedure_impl(
        &mut self,
        name: impl AsRef<str>,
        params: Vec<String>,
        body: &str,
        is_macro: bool,
    ) -> Result<(), VmError> {
        let name = name.as_ref();
        let name_symbol = self.interner.intern(name);
        let param_symbols: Vec<Symbol> = params
            .iter()
            .map(|param| self.interner.intern(param))
            .collect();
        self.arities.insert(name, Arity::Exact(param_symbols.len()));
        let program =
            parse_source(body, &mut self.interner, &self.arities).map_err(vm_error_from_parse)?;
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
                is_macro,
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
                    output_to,
                } => {
                    let args = pop_args(&mut stack, *argc)?;
                    match self.call(*callee, args, *expects_value)? {
                        PrimitiveResult::Value(value) => stack.push(value),
                        PrimitiveResult::NoValue => {
                            if *expects_value {
                                let consumer = output_to
                                    .as_ref()
                                    .map(|target| output_target_name(target, &self.interner))
                                    .unwrap_or_else(|| "a value".to_string());
                                return Err(self.error_with_code(
                                    5,
                                    format!(
                                        "{} didn't output to {}",
                                        self.interner.spelling(*callee),
                                        consumer
                                    ),
                                ));
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
                        return Err(self.error_with_code(
                            9,
                            format!(
                                "You don't say what to do with {}",
                                value.show(&self.interner)
                            ),
                        ));
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
            .ok_or_else(|| self.error_with_code(11, format!("{name} has no value")))
    }

    fn define_procedures_in_source(&mut self, source: &str) -> Result<String, VmError> {
        let mut runnable = Vec::new();
        let mut lines = source.lines().peekable();

        while let Some(line) = lines.next() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !starts_with_logo_word(trimmed, "to") && !starts_with_logo_word(trimmed, ".macro") {
                runnable.push(line.to_string());
                continue;
            }

            let (name, params, is_macro) = parse_to_header(trimmed)?;
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
            if is_macro {
                self.define_macro(name, params, &body.join("\n"))?;
            } else {
                self.define_procedure(name, params, &body.join("\n"))?;
            }
        }

        Ok(runnable.join("\n"))
    }

    fn call(
        &mut self,
        callee: Symbol,
        args: Vec<Value>,
        expects_value: bool,
    ) -> Result<PrimitiveResult, VmError> {
        let name = self.interner.canonical_spelling(callee).to_string();
        self.call_stack.push(name.clone());
        let result = match name.as_str() {
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
            "rank" => self.rank(args),
            "ranpick" => self.ranpick(args),
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
            "ascii" => self.ascii(args),
            "char" => self.char_(args),
            "lowercase" => self.lowercase(args),
            "rev" => self.rev(args),
            "setread" => self.setread(args),
            "setwrite" => self.setwrite(args),
            "openread" => self.openread(args),
            "openwrite" => self.openwrite(args),
            "openappend" => self.openappend(args),
            "close" => self.close(args),
            "reader" => self.reader(args),
            "writer" => self.writer(args),
            "dribble" => self.dribble_command(args),
            "nodribble" => self.nodribble(args),
            "readchar" | "rc" => self.readchar(args),
            "readlist" | "rl" => self.readlist(args),
            "readword" | "rw" => self.readword(args),
            "keyp" => self.keyp(args),
            "joy" => self.joy(args),
            "joyb" => self.joyb(args),
            "paddle" => self.paddle(args),
            "paddleb" => self.paddleb(args),
            "timeout" => self.timeout(args),
            "textscreen" | "ts" => self.textscreen(args),
            "splitscreen" | "ss" => self.splitscreen(args),
            "fullscreen" | "fs" => self.fullscreen(args),
            "setcursor" => self.setcursor(args),
            "setenv" => self.setenv(args),
            "make" | "name" => self.make(args),
            "thing" => self.thing(args),
            "local" => self.local(args),
            "namep" => self.namep(args),
            "wordp" => self.wordp(args),
            "realwordp" => self.realwordp(args),
            "listp" => self.listp(args),
            "numberp" => self.numberp(args),
            "intp" => self.intp(args),
            "decimalp" => self.decimalp(args),
            "evenp" => self.evenp(args),
            "divisorp" => self.divisorp(args),
            "factorial" => self.factorial(args),
            "definedp" | "defined?" => self.definedp(args),
            "primitivep" | "primitive?" => self.primitivep(args),
            "text" => self.text(args),
            "fulltext" => self.fulltext(args),
            "copydef" => self.copydef(args),
            "define" => self.define_from_data(args),
            ".defmacro" => self.defmacro(args),
            "macrop" | "macro?" => self.macrop(args),
            "macroexpand" => self.macroexpand(args),
            "po" => self.po(args),
            "poall" => self.poall(args),
            "pons" => self.pons(args),
            "pops" => self.pops(args),
            "pots" => self.pots(args),
            "popls" => self.popls(args),
            "edit" | "ed" => self.edit(args),
            ".primitives" => self.primitives_command(args),
            "erase" | "er" => self.erase(args),
            "ern" => self.ern(args),
            "erns" => self.erns(args),
            "erps" => self.erps(args),
            "erpl" => self.erpl(args),
            "erall" => self.erall(args),
            "bury" => self.bury(args),
            "unbury" => self.unbury(args),
            "buriedp" => self.buriedp(args),
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
            "setlabelheight" => self.turtle_setlabelheight(args),
            "label" => self.turtle_label(args),
            "fill" => self.turtle_fill(args),
            "hideturtle" | "ht" => self.turtle_hide(args),
            "init.turtle" => self.init_turtle(args),
            "showturtle" | "st" => self.turtle_show(args),
            "shownp" => self.turtle_shownp(args),
            "pos" => self.turtle_pos(args),
            "heading" => self.turtle_heading(args),
            "xcor" => self.turtle_xcor(args),
            "ycor" => self.turtle_ycor(args),
            "tell" => self.dyn_tell(args),
            "ask" => self.dyn_ask(args),
            "each" => self.dyn_each(args),
            "who" => self.dyn_who(args),
            "setvelocity" => self.dyn_setvelocity(args),
            "setspeed" => self.dyn_setspeed(args),
            "setshape" => self.dyn_setshape(args),
            "touching" => self.dyn_touching(args),
            "over" => self.over(args),
            "when" => self.dyn_when(args),
            "toot" => self.toot(args),
            "output" | "op" => self.output_control(args),
            "stop" => {
                expect_arity(&name, &args, 0).map(|()| PrimitiveResult::Control(ControlFlow::Stop))
            }
            _ => self.call_user_procedure(&name, args, expects_value),
        };
        let result = result.map_err(|error| self.contextualize_error(error));
        self.call_stack.pop();
        result
    }

    fn call_user_procedure(
        &mut self,
        name: &str,
        args: Vec<Value>,
        expects_value: bool,
    ) -> Result<PrimitiveResult, VmError> {
        let procedure = self
            .procedures
            .get(name)
            .cloned()
            .ok_or_else(|| VmError::new(format!("I don't know how to {name}")))?;
        expect_arity(name, &args, procedure.params.len())?;

        let control = self.run_procedure_body(&procedure, args)?;

        if procedure.is_macro() {
            return self.expand_and_run_macro(name, control, expects_value);
        }

        match control {
            ControlFlow::None | ControlFlow::Stop => Ok(PrimitiveResult::NoValue),
            ControlFlow::Output(value) => Ok(PrimitiveResult::Value(value)),
            ControlFlow::Continue => Ok(PrimitiveResult::Control(ControlFlow::Continue)),
            ControlFlow::Throw { tag, value } => {
                Ok(PrimitiveResult::Control(ControlFlow::Throw { tag, value }))
            }
        }
    }

    fn run_procedure_body(
        &mut self,
        procedure: &Procedure,
        args: Vec<Value>,
    ) -> Result<ControlFlow, VmError> {
        self.env.push_frame();
        for (param, value) in procedure.params().iter().zip(args) {
            let name = self.interner.spelling(*param).to_string();
            self.env.define_local(name, value);
        }
        let result = self.run(procedure.chunk());
        self.env.pop_frame();
        Ok(result?.control)
    }

    fn expand_and_run_macro(
        &mut self,
        name: &str,
        control: ControlFlow,
        expects_value: bool,
    ) -> Result<PrimitiveResult, VmError> {
        let expansion = match control {
            ControlFlow::Output(value) => value,
            ControlFlow::Throw { tag, value } => {
                return Ok(PrimitiveResult::Control(ControlFlow::Throw { tag, value }))
            }
            ControlFlow::Continue => {
                return Ok(PrimitiveResult::Control(ControlFlow::Continue));
            }
            ControlFlow::None | ControlFlow::Stop => {
                return Err(VmError::new(format!(
                    "{name} is a macro and must output an instruction list"
                )))
            }
        };
        let list = list_input(&expansion, &name.to_ascii_uppercase())?;
        let source = macro_list_to_source(list, &self.interner, &self.arities);
        let program = parse_source(&source, &mut self.interner, &self.arities)
            .map_err(vm_error_from_parse)?;
        let compiler = Compiler::new();
        if expects_value {
            let chunk = compiler
                .compile_program(&program)
                .map_err(|error| VmError::new(error.to_string()))?;
            let result = self.run(&chunk)?;
            match result.control {
                ControlFlow::Throw { tag, value } => {
                    Ok(PrimitiveResult::Control(ControlFlow::Throw { tag, value }))
                }
                ControlFlow::Continue => Ok(PrimitiveResult::Control(ControlFlow::Continue)),
                ControlFlow::Output(value) => Ok(PrimitiveResult::Value(value)),
                ControlFlow::Stop => Err(VmError::new(format!(
                    "{name} expansion stopped without output"
                ))),
                ControlFlow::None => result
                    .stack
                    .last()
                    .cloned()
                    .map(PrimitiveResult::Value)
                    .ok_or_else(|| {
                        VmError::new(format!("{name} expansion did not output a value"))
                    }),
            }
        } else {
            let chunk = compiler
                .compile_effect_program(&program)
                .map_err(|error| VmError::new(error.to_string()))?;
            match self.run(&chunk)?.control {
                ControlFlow::None => Ok(PrimitiveResult::NoValue),
                control => Ok(PrimitiveResult::Control(control)),
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
            Value::Word(symbol) => {
                let text = drop_first_char(self.interner.spelling(*symbol));
                Value::word(&mut self.interner, text)
            }
            Value::BareWord(symbol) => {
                let text = drop_first_char(self.interner.spelling(*symbol));
                Value::bare_word(&mut self.interner, text)
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
            Value::Word(symbol) => {
                let text = drop_last_char(self.interner.spelling(*symbol));
                Value::word(&mut self.interner, text)
            }
            Value::BareWord(symbol) => {
                let text = drop_last_char(self.interner.spelling(*symbol));
                Value::bare_word(&mut self.interner, text)
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

    fn rank(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("rank", &args, 1)?;
        Ok(PrimitiveResult::Value(Value::number(
            rank_value(&args[0]) as f64
        )))
    }

    fn ranpick(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("ranpick", &args, 1)?;
        match &args[0] {
            Value::List(list) => {
                let values = list_values(list);
                if values.is_empty() {
                    return Err(VmError::new("RANPICK of empty list"));
                }
                let index = (self.next_random_u64() as usize) % values.len();
                Ok(PrimitiveResult::Value(values[index].clone()))
            }
            Value::Array(array) => {
                let values = list_values(&array.to_list());
                if values.is_empty() {
                    return Err(VmError::new("RANPICK of empty array"));
                }
                let index = (self.next_random_u64() as usize) % values.len();
                Ok(PrimitiveResult::Value(values[index].clone()))
            }
            Value::Word(symbol) | Value::BareWord(symbol) => {
                let text = self.interner.spelling(*symbol).to_string();
                let random = self.next_random_u64();
                ranpick_text(&text, &mut self.interner, random)
            }
            Value::Number(number) => {
                let text = Value::Number(*number).show(&self.interner);
                let random = self.next_random_u64();
                ranpick_text(&text, &mut self.interner, random)
            }
        }
    }

    fn dot(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("dot", &args, 1)?;
        let target = point_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.draw_dot(id, target);
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
            source.push_str(&procedure_definition_text(procedure, &self.interner));
            if index + 1 < procedures.len() {
                source.push('\n');
            }
        }
        fs::write(&path, source)
            .map_err(|error| VmError::new(format!("{}: {error}", path.display())))?;
        Ok(PrimitiveResult::NoValue)
    }

    fn ascii(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("ascii", &args, 1)?;
        let text = source_text_input(&args[0], &self.interner);
        let Some(ch) = text.chars().next() else {
            return Err(VmError::new("ASCII of empty word"));
        };
        let code = u32::from(ch);
        if code > 255 {
            return Err(VmError::new("ASCII input must be a Latin-1 character"));
        }
        Ok(PrimitiveResult::Value(Value::number(code as f64)))
    }

    fn char_(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("char", &args, 1)?;
        let code = ranged_byte_input(&args[0], &self.interner, "CHAR", 255)?;
        Ok(PrimitiveResult::Value(Value::word(
            &mut self.interner,
            char::from(code).to_string(),
        )))
    }

    fn lowercase(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("lowercase", &args, 1)?;
        let text = source_text_input(&args[0], &self.interner);
        Ok(PrimitiveResult::Value(Value::word(
            &mut self.interner,
            text.to_ascii_lowercase(),
        )))
    }

    fn rev(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("rev", &args, 1)?;
        let value = match &args[0] {
            Value::List(list) => {
                let mut values = list.iter().cloned().collect::<Vec<_>>();
                values.reverse();
                Value::List(List::from_values(values))
            }
            Value::Array(array) => {
                let mut values = array.to_list().iter().cloned().collect::<Vec<_>>();
                values.reverse();
                Value::List(List::from_values(values))
            }
            Value::Word(symbol) => {
                let reversed = self
                    .interner
                    .spelling(*symbol)
                    .chars()
                    .rev()
                    .collect::<String>();
                Value::word(&mut self.interner, reversed)
            }
            Value::Number(number) => {
                let reversed = Value::Number(*number)
                    .show(&self.interner)
                    .chars()
                    .rev()
                    .collect::<String>();
                Value::word(&mut self.interner, reversed)
            }
            Value::BareWord(symbol) => {
                let reversed = self
                    .interner
                    .spelling(*symbol)
                    .chars()
                    .rev()
                    .collect::<String>();
                Value::bare_word(&mut self.interner, reversed)
            }
        };
        Ok(PrimitiveResult::Value(value))
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

        if self
            .dribble
            .as_ref()
            .is_some_and(|path| path.to_string_lossy() == key)
        {
            self.dribble = None;
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
        let name = self
            .read_stream
            .as_ref()
            .map(|stream| stream.path.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok(PrimitiveResult::Value(Value::word(
            &mut self.interner,
            name,
        )))
    }

    fn writer(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("writer", &args, 0)?;
        Ok(PrimitiveResult::Value(Value::word(
            &mut self.interner,
            self.current_write.clone().unwrap_or_default(),
        )))
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
        let values = lex(&line)
            .map_err(|error| VmError::new(error.to_string()))?
            .into_iter()
            .filter_map(|token| token_to_data_value(token.kind, &mut self.interner))
            .collect::<Vec<_>>();
        Ok(PrimitiveResult::Value(Value::List(List::from_values(
            values,
        ))))
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

    fn keyp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("keyp", &args, 0)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(!self.outside_world.key_buffer.is_empty()),
        ))
    }

    fn joy(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("joy", &args, 1)?;
        let index = ranged_device_index_input(&args[0], &self.interner, "JOY", 4)?;
        Ok(PrimitiveResult::Value(Value::number(
            self.outside_world.joystick_directions[index] as f64,
        )))
    }

    fn joyb(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("joyb", &args, 1)?;
        let index = ranged_device_index_input(&args[0], &self.interner, "JOYB", 4)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(self.outside_world.joystick_buttons[index]),
        ))
    }

    fn paddle(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("paddle", &args, 1)?;
        let index = ranged_device_index_input(&args[0], &self.interner, "PADDLE", 8)?;
        Ok(PrimitiveResult::Value(Value::number(
            self.outside_world.paddle_positions[index] as f64,
        )))
    }

    fn paddleb(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("paddleb", &args, 1)?;
        let index = ranged_device_index_input(&args[0], &self.interner, "PADDLEB", 8)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(self.outside_world.paddle_buttons[index]),
        ))
    }

    fn timeout(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("timeout", &args, 1)?;
        self.outside_world.timeout_sixtieths =
            non_negative_integer_input(&args[0], &self.interner, "TIMEOUT")?;
        Ok(PrimitiveResult::NoValue)
    }

    fn textscreen(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("textscreen", &args, 0)?;
        self.outside_world.text_screen_mode = TextScreenMode::TextScreen;
        Ok(PrimitiveResult::NoValue)
    }

    fn splitscreen(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("splitscreen", &args, 0)?;
        self.outside_world.text_screen_mode = TextScreenMode::SplitScreen;
        Ok(PrimitiveResult::NoValue)
    }

    fn fullscreen(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("fullscreen", &args, 0)?;
        self.outside_world.text_screen_mode = TextScreenMode::FullScreen;
        Ok(PrimitiveResult::NoValue)
    }

    fn setcursor(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setcursor", &args, 2)?;
        self.outside_world.text_cursor = (
            non_negative_integer_input(&args[0], &self.interner, "SETCURSOR")?,
            non_negative_integer_input(&args[1], &self.interner, "SETCURSOR")?,
        );
        Ok(PrimitiveResult::NoValue)
    }

    fn setenv(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setenv", &args, 4)?;
        self.outside_world.sound_envelope = [
            ranged_byte_input(&args[0], &self.interner, "SETENV", 255)?,
            ranged_byte_input(&args[1], &self.interner, "SETENV", 255)?,
            ranged_byte_input(&args[2], &self.interner, "SETENV", 255)?,
            ranged_byte_input(&args[3], &self.interner, "SETENV", 255)?,
        ];
        Ok(PrimitiveResult::NoValue)
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
            Value::Word(_) | Value::Number(_)
        ))))
    }

    fn realwordp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("realwordp", &args, 1)?;
        Ok(PrimitiveResult::Value(
            self.logo_bool(matches!(args[0], Value::Word(_))),
        ))
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

    fn evenp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("evenp", &args, 1)?;
        let Some(number) = args[0].as_number(&self.interner) else {
            return Ok(PrimitiveResult::Value(self.logo_bool(false)));
        };
        Ok(PrimitiveResult::Value(self.logo_bool(
            LogoNumber::new(number).is_integerish() && (number as i64) % 2 == 0,
        )))
    }

    fn divisorp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("divisorp", &args, 2)?;
        let divisor = number_input(&args[0], &self.interner)?;
        let dividend = number_input(&args[1], &self.interner)?;
        if divisor == 0.0 {
            return Err(VmError::new("DIVISORP first input must not be zero"));
        }
        Ok(PrimitiveResult::Value(
            self.logo_bool(dividend.rem_euclid(divisor) == 0.0),
        ))
    }

    fn factorial(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("factorial", &args, 1)?;
        let number = number_input(&args[0], &self.interner)?;
        if number < 0.0 || !LogoNumber::new(number).is_integerish() {
            return Err(VmError::new("FACTORIAL expects a nonnegative integer"));
        }
        let mut product = 1.0;
        for value in 1..=(number as u64) {
            product *= value as f64;
        }
        Ok(PrimitiveResult::Value(Value::number(product)))
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
        self.define_procedure_impl(new_name, params, &body_source, procedure.is_macro())?;
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

    fn defmacro(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity(".defmacro", &args, 3)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        let params = parameter_names_input(&args[1], &self.interner)?;
        let body_lines = define_body_input(&args[2], &self.interner, &self.arities)?;
        self.define_macro(name, params, &body_lines.join("\n"))?;
        Ok(PrimitiveResult::NoValue)
    }

    fn macrop(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("macrop", &args, 1)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        let is_macro = self
            .procedures
            .get(&name.to_ascii_lowercase())
            .map(|procedure| procedure.is_macro())
            .unwrap_or(false);
        Ok(PrimitiveResult::Value(self.logo_bool(is_macro)))
    }

    fn macroexpand(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("macroexpand", &args, 1)?;
        let list = list_input(&args[0], "MACROEXPAND")?;
        let mut items = list.iter().cloned();
        let head = items
            .next()
            .ok_or_else(|| VmError::new("MACROEXPAND requires a non-empty instruction list"))?;
        let name = variable_name_input(&head, &self.interner)?;
        let procedure = self
            .procedures
            .get(&name.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| VmError::new(format!("I don't know how to {name}")))?;
        if !procedure.is_macro() {
            return Err(VmError::new(format!("{name} is not a macro")));
        }
        let call_args: Vec<Value> = items.collect();
        expect_arity(&name, &call_args, procedure.params().len())?;

        match self.run_procedure_body(&procedure, call_args)? {
            ControlFlow::Output(value) => Ok(PrimitiveResult::Value(value)),
            ControlFlow::Continue => Ok(PrimitiveResult::Control(ControlFlow::Continue)),
            ControlFlow::Throw { tag, value } => {
                Ok(PrimitiveResult::Control(ControlFlow::Throw { tag, value }))
            }
            ControlFlow::None | ControlFlow::Stop => Err(VmError::new(format!(
                "{name} is a macro and must output an instruction list"
            ))),
        }
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
        self.write_property_list_listing();
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

    fn popls(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("popls", &args, 0)?;
        self.write_property_list_listing();
        Ok(PrimitiveResult::NoValue)
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
            self.buried_names.remove(&key);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn ern(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("ern", &args, 1)?;
        for name in local_names(&args[0], &self.interner)? {
            let key = name.to_ascii_lowercase();
            self.env.globals.remove(&key);
            self.buried_names.remove(&key);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn erns(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("erns", &args, 0)?;
        self.env.globals.clear();
        self.buried_names.retain(|name| {
            !self.procedures.contains_key(name) && !self.property_lists.contains_key(name)
        });
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
            self.buried_names.remove(&name);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn erpl(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("erpl", &args, 1)?;
        for name in local_names(&args[0], &self.interner)? {
            let key = name.to_ascii_lowercase();
            self.property_lists.remove(&key);
            self.buried_names.remove(&key);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn erall(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("erall", &args, 0)?;
        self.erns(vec![])?;
        self.erps(vec![])?;
        self.property_lists.clear();
        self.buried_names.clear();
        Ok(PrimitiveResult::NoValue)
    }

    fn bury(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("bury", &args, 1)?;
        for name in local_names(&args[0], &self.interner)? {
            self.buried_names.insert(name.to_ascii_lowercase());
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn unbury(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("unbury", &args, 1)?;
        for name in local_names(&args[0], &self.interner)? {
            self.buried_names.remove(&name.to_ascii_lowercase());
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn buriedp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("buriedp", &args, 1)?;
        let name = variable_name_input(&args[0], &self.interner)?;
        Ok(PrimitiveResult::Value(self.logo_bool(
            self.buried_names.contains(&name.to_ascii_lowercase()),
        )))
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
            .filter(|(name, _)| {
                !is_protected_workspace_procedure(name) && !self.buried_names.contains(*name)
            })
            .map(|(_, procedure)| procedure.clone())
            .collect::<Vec<_>>();
        procedures.sort_by_key(|procedure| {
            self.interner
                .canonical_spelling(procedure.name())
                .to_string()
        });
        procedures
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

    fn write_variable_listing(&mut self) {
        let mut names = self
            .env
            .globals
            .keys()
            .filter(|name| !self.buried_names.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
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

    fn write_property_list_listing(&mut self) {
        let mut names = self
            .property_lists
            .keys()
            .filter(|name| !self.buried_names.contains(*name))
            .cloned()
            .collect::<Vec<_>>();
        names.sort();
        for name in names {
            self.output.push_str(&name);
            self.output.push(' ');
            let value = self
                .property_lists
                .get(&name)
                .cloned()
                .map(|plist| {
                    let mut entries: Vec<_> = plist.into_iter().collect();
                    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
                    let mut values = Vec::new();
                    for (entry_name, entry_value) in entries {
                        values.push(Value::word(&mut self.interner, entry_name));
                        values.push(entry_value);
                    }
                    Value::List(List::from_values(values))
                })
                .unwrap_or_else(|| Value::List(List::empty()));
            self.output.push_str(&value.show(&self.interner));
            self.output.push('\n');
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
        let chunk = self.compile_list(list, CompileMode::Effect)?;
        self.run(&chunk)
    }

    fn execute_instruction_list_result(&mut self, list: &List) -> Result<RunResult, VmError> {
        let chunk = self.compile_list(list, CompileMode::Result)?;
        self.run(&chunk)
    }

    /// Instruction lists are immutable `Arc`-backed cons cells (see
    /// `List::pointer_identity`), so once a list's bytecode is compiled it can
    /// be reused for the lifetime of that list value with no invalidation
    /// needed. This matters because the same body list is re-executed on every
    /// iteration of `REPEAT`/`FOREVER`, every `WHEN` demon firing, etc. -
    /// without caching, each of those re-runs a full lex/parse/compile pass.
    fn compile_list(&mut self, list: &List, mode: CompileMode) -> Result<Chunk, VmError> {
        if let Some(key) = ChunkKey::for_list(list, mode) {
            if let Some(chunk) = self.chunk_cache.get(key) {
                return Ok(chunk.clone());
            }
            let chunk = compile_list_source(list, mode, &mut self.interner, &self.arities)?;
            self.chunk_cache.insert(key, chunk.clone());
            return Ok(chunk);
        }
        compile_list_source(list, mode, &mut self.interner, &self.arities)
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
            self.invoke_template_effect(&args[0], vec![value])?;
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
        expect_arity("cascade.2", &args, 4)?;
        self.cascade(vec![
            args[0].clone(),
            args[1].clone(),
            args[2].clone(),
            args[3].clone(),
        ])
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
                    self.call(symbol, values, false)
                } else {
                    self.env.push_frame();
                    self.bind_extra_template_bindings(extra_bindings);
                    let result = self.call(symbol, values, false);
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
                    self.call(symbol, values, true)
                } else {
                    self.env.push_frame();
                    self.bind_extra_template_bindings(extra_bindings);
                    let result = self.call(symbol, values, true);
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
            None => Err(self.error_with_code(25, "IFTRUE/IFFALSE without TEST")),
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
            None => Err(self.error_with_code(25, "IFTRUE/IFFALSE without TEST")),
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
                } if thrown_tag.equalp(&tag, &self.interner) => {
                    if is_error_catch_tag(&tag, &self.interner) {
                        let mut info = ErrorInfo::new(35, value.show(&self.interner));
                        if let Some(name) = self.current_error_context() {
                            info.procedure = name.to_string();
                        }
                        self.caught_error = Some(info);
                        Ok(PrimitiveResult::NoValue)
                    } else {
                        Ok(PrimitiveResult::Value(value))
                    }
                }
                ControlFlow::None | ControlFlow::Stop => Ok(PrimitiveResult::NoValue),
                ControlFlow::Output(value) => Ok(PrimitiveResult::Value(value)),
                ControlFlow::Continue => Ok(PrimitiveResult::Control(ControlFlow::Continue)),
                ControlFlow::Throw { tag, value } => {
                    Ok(PrimitiveResult::Control(ControlFlow::Throw { tag, value }))
                }
            },
            Err(error) if is_error_catch_tag(&tag, &self.interner) => {
                self.record_caught_error(&error);
                Ok(PrimitiveResult::NoValue)
            }
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
        let value = self
            .caught_error
            .take()
            .map(|info| info.to_value(&mut self.interner))
            .unwrap_or_else(|| Value::List(List::empty()));
        Ok(PrimitiveResult::Value(value))
    }

    fn pause(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pause", &args, 0)?;
        self.pause_depth += 1;
        let result = self.pause_loop();
        self.pause_depth -= 1;
        result
    }

    fn pause_loop(&mut self) -> Result<PrimitiveResult, VmError> {
        loop {
            let Some(line) = self.read_pause_line()? else {
                return Err(VmError::new("PAUSE aborted by end of input"));
            };
            if line.trim().is_empty() {
                continue;
            }

            match self.eval_source(&line) {
                Ok(result) => {
                    if !result.output.is_empty() {
                        print!("{}", result.output);
                        self.clear_output();
                        io::stdout()
                            .flush()
                            .map_err(|error| VmError::new(error.to_string()))?;
                    }
                    match result.control {
                        ControlFlow::None => {
                            for value in result.stack {
                                println!("{}", value.show(self.interner()));
                            }
                        }
                        ControlFlow::Continue => return Ok(PrimitiveResult::NoValue),
                        ControlFlow::Output(value) => {
                            return Ok(PrimitiveResult::Control(ControlFlow::Output(value)));
                        }
                        ControlFlow::Stop => {
                            return Ok(PrimitiveResult::Control(ControlFlow::Stop));
                        }
                        ControlFlow::Throw { tag, value } => {
                            return Ok(PrimitiveResult::Control(ControlFlow::Throw { tag, value }));
                        }
                    }
                }
                Err(error) => eprintln!("{error}"),
            }
        }
    }

    fn read_pause_line(&mut self) -> Result<Option<String>, VmError> {
        if let Some(line) = self.pause_inputs.pop_front() {
            return Ok(Some(line));
        }

        print!("pause> ");
        io::stdout()
            .flush()
            .map_err(|error| VmError::new(error.to_string()))?;
        let mut line = String::new();
        let bytes = io::stdin()
            .read_line(&mut line)
            .map_err(|error| VmError::new(error.to_string()))?;
        if bytes == 0 {
            Ok(None)
        } else {
            Ok(Some(line))
        }
    }

    fn continue_(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("continue", &args, 0)?;
        if self.pause_depth == 0 {
            Err(VmError::new("CONTINUE can only be used inside PAUSE"))
        } else {
            Ok(PrimitiveResult::Control(ControlFlow::Continue))
        }
    }

    /// The single turtle classic query primitives (POS, HEADING, ...) read
    /// from: the first member of the active TELL selection.
    fn current_turtle(&self) -> TurtleId {
        self.turtles.who().unwrap_or_else(|| TurtleId::new(0))
    }

    fn turtle_forward(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("forward", &args, 1)?;
        let distance = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.forward(id, distance);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_back(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("back", &args, 1)?;
        let distance = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.back(id, distance);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_left(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("left", &args, 1)?;
        let degrees = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.left(id, degrees);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_right(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("right", &args, 1)?;
        let degrees = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.right(id, degrees);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setxy(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setxy", &args, 2)?;
        let x = number_input(&args[0], &self.interner)?;
        let y = number_input(&args[1], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_xy(id, x, y);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setx(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setx", &args, 1)?;
        let x = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            let y = self
                .turtles
                .state(id)
                .expect("active turtle exists in store")
                .position
                .y;
            self.turtles.set_xy(id, x, y);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_sety(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("sety", &args, 1)?;
        let y = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            let x = self
                .turtles
                .state(id)
                .expect("active turtle exists in store")
                .position
                .x;
            self.turtles.set_xy(id, x, y);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setpos(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setpos", &args, 1)?;
        let point = point_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.goto(id, point);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setheading(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setheading", &args, 1)?;
        let heading = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_heading(id, heading);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_home(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("home", &args, 0)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.home(id);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_clearscreen(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("clearscreen", &args, 0)?;
        let ids = self.turtles.active().to_vec();
        self.turtles.clearscreen(&ids);
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_penup(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("penup", &args, 0)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_pen_down(id, false);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_pendown(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pendown", &args, 0)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_pen_down(id, true);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setpencolor(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setpencolor", &args, 1)?;
        let color = number_input(&args[0], &self.interner)? as u32;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_pen_color(id, color);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setpensize(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setpensize", &args, 1)?;
        let width = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_pen_size(id, width);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_setlabelheight(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setlabelheight", &args, 1)?;
        let height = number_input(&args[0], &self.interner)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_label_height(id, height);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_label(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("label", &args, 1)?;
        let text = args[0].show(&self.interner);
        for id in self.turtles.active().to_vec() {
            self.turtles.label(id, text.clone());
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_fill(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("fill", &args, 0)?;
        for id in self.turtles.active().to_vec() {
            let color = self
                .turtles
                .state(id)
                .expect("active turtle exists in store")
                .pen_color;
            self.turtles.fill(id, color);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_hide(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("hideturtle", &args, 0)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_visible(id, false);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_show(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("showturtle", &args, 0)?;
        for id in self.turtles.active().to_vec() {
            self.turtles.set_visible(id, true);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn init_turtle(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("init.turtle", &args, 0)?;
        let ids = self.turtles.active().to_vec();
        self.turtles.clearscreen(&ids);
        for id in ids {
            self.turtles.set_visible(id, true);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn turtle_shownp(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("shownp", &args, 0)?;
        let id = self.current_turtle();
        let visible = self
            .turtles
            .state(id)
            .expect("active turtle exists in store")
            .visible;
        Ok(PrimitiveResult::Value(self.logo_bool(visible)))
    }

    fn turtle_pos(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("pos", &args, 0)?;
        let id = self.current_turtle();
        let position = self
            .turtles
            .state(id)
            .expect("active turtle exists in store")
            .position;
        Ok(PrimitiveResult::Value(Value::List(List::from_values([
            Value::number(position.x),
            Value::number(position.y),
        ]))))
    }

    fn turtle_heading(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("heading", &args, 0)?;
        let id = self.current_turtle();
        let heading = self
            .turtles
            .state(id)
            .expect("active turtle exists in store")
            .heading;
        Ok(PrimitiveResult::Value(Value::number(heading)))
    }

    fn turtle_xcor(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("xcor", &args, 0)?;
        let id = self.current_turtle();
        let position = self
            .turtles
            .state(id)
            .expect("active turtle exists in store")
            .position;
        Ok(PrimitiveResult::Value(Value::number(position.x)))
    }

    fn turtle_ycor(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("ycor", &args, 0)?;
        let id = self.current_turtle();
        let position = self
            .turtles
            .state(id)
            .expect("active turtle exists in store")
            .position;
        Ok(PrimitiveResult::Value(Value::number(position.y)))
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

    fn turtle_id_from_value(&self, value: &Value) -> Result<TurtleId, VmError> {
        Ok(TurtleId::new(number_input(value, &self.interner)? as usize))
    }

    fn turtle_ids_input(&self, value: &Value) -> Result<Vec<TurtleId>, VmError> {
        match value {
            Value::List(list) => list
                .iter()
                .map(|item| self.turtle_id_from_value(item))
                .collect(),
            _ => Ok(vec![self.turtle_id_from_value(value)?]),
        }
    }

    fn dyn_tell(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("tell", &args, 1)?;
        let ids = self.turtle_ids_input(&args[0])?;
        if ids.is_empty() {
            return Err(VmError::new("TELL requires at least one turtle"));
        }
        self.turtles.tell_many(ids);
        Ok(PrimitiveResult::NoValue)
    }

    fn dyn_ask(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("ask", &args, 2)?;
        let id = self.turtle_id_from_value(&args[0])?;
        let list = list_input(&args[1], "ASK")?.clone();
        let previous = self.turtles.active().to_vec();
        self.turtles.tell_one(id);
        let control = self.execute_instruction_list(&list)?;
        self.turtles.tell_many(previous);
        match control {
            ControlFlow::None => Ok(PrimitiveResult::NoValue),
            control => Ok(PrimitiveResult::Control(control)),
        }
    }

    fn dyn_each(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("each", &args, 1)?;
        let list = list_input(&args[0], "EACH")?.clone();
        let ids = self.turtles.active().to_vec();
        for id in ids {
            self.turtles.tell_one(id);
            match self.execute_instruction_list(&list)? {
                ControlFlow::None => {}
                control => return Ok(PrimitiveResult::Control(control)),
            }
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn dyn_who(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("who", &args, 0)?;
        let ids = self.turtles.active().to_vec();
        Ok(PrimitiveResult::Value(Value::List(List::from_values(
            ids.into_iter().map(|id| Value::number(id.index() as f64)),
        ))))
    }

    fn dyn_setvelocity(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setvelocity", &args, 2)?;
        let dx = number_input(&args[0], &self.interner)?;
        let dy = number_input(&args[1], &self.interner)?;
        let ids = self.turtles.active().to_vec();
        for id in ids {
            self.turtles.set_velocity(id, Point::new(dx, dy));
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn dyn_setspeed(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setspeed", &args, 1)?;
        let speed = number_input(&args[0], &self.interner)?;
        let ids = self.turtles.active().to_vec();
        for id in ids {
            self.turtles.set_speed(id, speed);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn dyn_setshape(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("setshape", &args, 2)?;
        let name = source_text_input(&args[0], &self.interner);
        let radius = number_input(&args[1], &self.interner)?;
        let ids = self.turtles.active().to_vec();
        for id in ids {
            self.turtles.set_shape(id, name.clone(), radius);
        }
        Ok(PrimitiveResult::NoValue)
    }

    fn dyn_touching(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("touching", &args, 2)?;
        let a = self.turtle_id_from_value(&args[0])?;
        let b = self.turtle_id_from_value(&args[1])?;
        self.turtles.ensure(a);
        self.turtles.ensure(b);
        let radius_a = self.turtles.collision_radius(a).unwrap_or(8.0);
        let radius_b = self.turtles.collision_radius(b).unwrap_or(8.0);
        let touching = collision::touching(&self.turtles, a, b, (radius_a + radius_b) / 2.0);
        Ok(PrimitiveResult::Value(self.logo_bool(touching)))
    }

    fn over(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("over", &args, 1)?;
        let color = number_input(&args[0], &self.interner)? as u32;
        let over = self
            .turtles
            .active()
            .iter()
            .copied()
            .any(|id| turtle_over_color(self.turtles.events(), &self.turtles, id, color));
        Ok(PrimitiveResult::Value(self.logo_bool(over)))
    }

    fn dyn_when(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("when", &args, 2)?;
        let condition = self.parse_demon_condition(list_input(&args[0], "WHEN")?)?;
        let body = list_input(&args[1], "WHEN")?.clone();
        self.demons.register(condition, body);
        Ok(PrimitiveResult::NoValue)
    }

    fn toot(&mut self, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        expect_arity("toot", &args, 4)?;
        self.last_toot = Some([
            ranged_byte_input(&args[0], &self.interner, "TOOT", 255)?,
            ranged_byte_input(&args[1], &self.interner, "TOOT", 255)?,
            ranged_byte_input(&args[2], &self.interner, "TOOT", 255)?,
            ranged_byte_input(&args[3], &self.interner, "TOOT", 255)?,
        ]);
        Ok(PrimitiveResult::NoValue)
    }

    fn parse_demon_condition(&self, list: &List) -> Result<DemonCondition, VmError> {
        let values = list_values(list);
        let head = values
            .first()
            .ok_or_else(|| VmError::new("WHEN condition must name TOUCHING, EDGE, or OVERCOLOR"))?;
        let symbol = match head {
            Value::Word(symbol) | Value::BareWord(symbol) => symbol,
            _ => return Err(VmError::new("WHEN condition must start with a word")),
        };
        match self.interner.canonical_spelling(*symbol) {
            "touching" if values.len() == 3 => {
                let a = self.turtle_id_from_value(&values[1])?;
                let b = self.turtle_id_from_value(&values[2])?;
                Ok(DemonCondition::Touching(a, b))
            }
            "edge" if values.len() == 1 => Ok(DemonCondition::Edge(None)),
            "edge" if values.len() == 2 => Ok(DemonCondition::Edge(Some(
                self.turtle_id_from_value(&values[1])?,
            ))),
            "over" | "overcolor" if values.len() == 2 => {
                let color = number_input(&values[1], &self.interner)? as u32;
                Ok(DemonCondition::OverColor(color))
            }
            other => Err(VmError::new(format!(
                "unrecognized WHEN condition: {other}"
            ))),
        }
    }
}

fn turtle_over_color(
    events: &[TurtleEvent],
    store: &TurtleStore,
    turtle: TurtleId,
    color: u32,
) -> bool {
    let Some(position) = store.state(turtle).map(|state| state.position) else {
        return false;
    };
    let start = events
        .iter()
        .rposition(|event| matches!(event, TurtleEvent::Clear))
        .map(|index| index + 1)
        .unwrap_or(0);
    events[start..].iter().any(|event| match event {
        TurtleEvent::Line {
            from,
            to,
            color: line_color,
            width,
            ..
        } => *line_color == color && point_near_segment(position, *from, *to, *width / 2.0),
        _ => false,
    })
}

fn point_near_segment(point: Point, a: Point, b: Point, tolerance: f64) -> bool {
    let segment_dx = b.x - a.x;
    let segment_dy = b.y - a.y;
    let segment_length_squared = segment_dx * segment_dx + segment_dy * segment_dy;
    if segment_length_squared == 0.0 {
        return distance_squared(point, a) <= tolerance * tolerance;
    }

    let projection =
        ((point.x - a.x) * segment_dx + (point.y - a.y) * segment_dy) / segment_length_squared;
    let projection = projection.clamp(0.0, 1.0);
    let closest = Point::new(a.x + segment_dx * projection, a.y + segment_dy * projection);
    distance_squared(point, closest) <= tolerance * tolerance
}

fn distance_squared(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn demon_event_turtles(event: &DemonEvent) -> Vec<TurtleId> {
    match event {
        DemonEvent::Touching(pair) => vec![pair.a, pair.b],
        DemonEvent::Edge(contact) => vec![contact.turtle],
        DemonEvent::OverColor { turtle, .. } => vec![*turtle],
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
        Err(VmError::new(format!("not enough inputs to {name}",)))
    }
}

fn output_target_name(target: &OutputTarget, interner: &Interner) -> String {
    match target {
        OutputTarget::Procedure(symbol) => interner.spelling(*symbol).to_string(),
        OutputTarget::Infix(op) => op.to_string(),
    }
}

fn infer_error_info(message: &str) -> Option<ErrorInfo> {
    if message.contains(" didn't output to ") || message.contains(" didn't output a value") {
        return Some(ErrorInfo::new(5, message));
    }
    if let Some(name) = message.strip_prefix("not enough inputs to ") {
        return Some(ErrorInfo::new(6, format!("not enough inputs to {name}")));
    }
    if message.starts_with("You don't say what to do with ") {
        return Some(ErrorInfo::new(9, message));
    }
    if message.ends_with(" has no value") {
        return Some(ErrorInfo::new(11, message));
    }
    if message.starts_with("I don't know how to ") {
        return Some(ErrorInfo::new(13, message));
    }
    if message == "IFTRUE/IFFALSE without TEST" || message == "TEST has not been run" {
        return Some(ErrorInfo::new(25, "IFTRUE/IFFALSE without TEST"));
    }
    None
}

fn vm_error_from_parse(error: ParseError) -> VmError {
    let info = infer_error_info(&error.message).map(|mut info| {
        info.message = error.message.clone();
        info
    });
    VmError {
        message: error.to_string(),
        info,
    }
}

fn number_input(value: &Value, interner: &Interner) -> Result<f64, VmError> {
    value
        .as_number(interner)
        .ok_or_else(|| VmError::new(format!("{} is not a number", value.show(interner))))
}

fn variable_name_input(value: &Value, interner: &Interner) -> Result<String, VmError> {
    match value {
        Value::Word(symbol) | Value::BareWord(symbol) => Ok(interner.spelling(*symbol).to_string()),
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
        Value::Word(symbol) | Value::BareWord(symbol) => interner.spelling(*symbol).to_string(),
        _ => value.show(interner),
    }
}

fn ranged_byte_input(
    value: &Value,
    interner: &Interner,
    primitive: &str,
    max: u8,
) -> Result<u8, VmError> {
    let number = number_input(value, interner)?;
    if !number.is_finite() || number.fract() != 0.0 || number < 0.0 || number > f64::from(max) {
        return Err(VmError::new(format!(
            "{primitive} expected an integer between 0 and {max}, got {}",
            value.show(interner)
        )));
    }
    Ok(number as u8)
}

fn non_negative_integer_input(
    value: &Value,
    interner: &Interner,
    name: &str,
) -> Result<usize, VmError> {
    let number = number_input(value, interner)?;
    if number < 0.0 || number.fract() != 0.0 {
        return Err(VmError::new(format!(
            "{name} requires a non-negative integer input"
        )));
    }
    Ok(number as usize)
}

fn ranged_device_index_input(
    value: &Value,
    interner: &Interner,
    name: &str,
    device_count: usize,
) -> Result<usize, VmError> {
    let index = non_negative_integer_input(value, interner, name)?;
    if index >= device_count {
        return Err(VmError::new(format!(
            "{name} index {index} is out of range"
        )));
    }
    Ok(index)
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

fn parse_to_header(line: &str) -> Result<(String, Vec<String>, bool), VmError> {
    let tokens = lex(line).map_err(|error| VmError::new(error.to_string()))?;
    let mut iter = tokens.into_iter();
    let is_macro = match iter.next().map(|token| token.kind) {
        Some(TokenKind::Word(word)) if word.eq_ignore_ascii_case("to") => false,
        Some(TokenKind::Word(word)) if word.eq_ignore_ascii_case(".macro") => true,
        _ => return Err(VmError::new("expected TO or .MACRO header")),
    };

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
    Ok((name, params, is_macro))
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
        ControlFlow::Continue => Err(VmError::new("CONTINUE can only be used inside PAUSE")),
        ControlFlow::Throw { tag, value } => Ok(Value::List(List::from_values([tag, value]))),
    }
}

fn rank_value(value: &Value) -> usize {
    match value {
        Value::List(list) => 1 + list.iter().map(rank_value).max().unwrap_or(0),
        Value::Array(array) => 1 + array.to_list().iter().map(rank_value).max().unwrap_or(0),
        Value::Word(_) | Value::BareWord(_) | Value::Number(_) => 0,
    }
}

fn ranpick_text(
    text: &str,
    interner: &mut Interner,
    random: u64,
) -> Result<PrimitiveResult, VmError> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Err(VmError::new("RANPICK of empty word"));
    }
    let ch = chars[(random as usize) % chars.len()];
    Ok(PrimitiveResult::Value(Value::word(
        interner,
        ch.to_string(),
    )))
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
        Value::Word(symbol) if interner.canonical_spelling(*symbol).eq("error")
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
        Value::List(list) => format!(
            "[{}]",
            list_to_source(list, interner, &ArityTable::default())
        ),
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

fn edit_temp_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    path.push(format!("dynalogo-edit-{nonce}.lgo"));
    path
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
        "rank",
        "ranpick",
        "ascii",
        "char",
        "lowercase",
        "rev",
        "wordp",
        "realwordp",
        "listp",
        "numberp",
        "intp",
        "decimalp",
        "evenp",
        "divisorp",
        "factorial",
        "print",
        "pr",
        "show",
        "type",
        "load",
        "save",
        "setread",
        "setwrite",
        "openread",
        "openwrite",
        "openappend",
        "close",
        "reader",
        "writer",
        "dribble",
        "nodribble",
        "readchar",
        "rc",
        "readlist",
        "rl",
        "readword",
        "rw",
        "keyp",
        "joy",
        "joyb",
        "paddle",
        "paddleb",
        "timeout",
        "textscreen",
        "ts",
        "splitscreen",
        "ss",
        "fullscreen",
        "fs",
        "setcursor",
        "setenv",
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
        "define",
        ".defmacro",
        "macrop",
        "macro?",
        "macroexpand",
        "po",
        "poall",
        "pons",
        "pops",
        "pots",
        "popls",
        "edit",
        "ed",
        ".primitives",
        "erase",
        "er",
        "ern",
        "erns",
        "erps",
        "erpl",
        "erall",
        "bury",
        "unbury",
        "buriedp",
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
        "setlabelheight",
        "label",
        "fill",
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
        "tell",
        "ask",
        "each",
        "who",
        "setvelocity",
        "setspeed",
        "setshape",
        "touching",
        "over",
        "when",
        "toot",
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

fn compile_list_source(
    list: &List,
    mode: CompileMode,
    interner: &mut Interner,
    arities: &ArityTable,
) -> Result<Chunk, VmError> {
    let source = list_to_source(list, interner, arities);
    let program = parse_source(&source, interner, arities).map_err(vm_error_from_parse)?;
    let compiler = Compiler::new();
    match mode {
        CompileMode::Effect => compiler.compile_effect_program(&program),
        CompileMode::Result => compiler.compile_program(&program),
    }
    .map_err(|error| VmError::new(error.to_string()))
}

fn list_to_source(list: &List, interner: &Interner, arities: &ArityTable) -> String {
    let values: Vec<&Value> = list.iter().collect();
    let mut rendered = Vec::new();
    let mut index = 0;
    while index < values.len() {
        let (expr, consumed) = value_expr_to_source(&values[index..], interner, arities);
        rendered.push(expr);
        index += consumed.max(1);
    }
    rendered.join(" ")
}

fn macro_list_to_source(list: &List, interner: &Interner, arities: &ArityTable) -> String {
    list.iter()
        .map(|value| match value {
            Value::List(inner) => format!("[{}]", macro_list_to_source(inner, interner, arities)),
            Value::Word(symbol) => {
                let spelling = interner.spelling(*symbol);
                if spelling.starts_with(':')
                    || arities.get(spelling).is_some()
                    || is_operator_word(spelling)
                {
                    spelling.to_string()
                } else {
                    format!("\"{spelling}")
                }
            }
            Value::BareWord(symbol) => {
                let spelling = interner.spelling(*symbol);
                if let Some(binding) = template_binding_name(spelling) {
                    format!(":{binding}")
                } else {
                    spelling.to_string()
                }
            }
            _ => value.show(interner),
        })
        .collect::<Vec<_>>()
        .join(" ")
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
                    let (arg, arg_consumed) =
                        value_expr_to_source(&values[consumed..], interner, arities);
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
        assert_eq!(
            vm.turtles().state(TurtleId::new(0)).unwrap().position,
            Point::new(10.0, 20.0)
        );
        let line = vm.turtles().events().iter().find_map(|event| match event {
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
    fn repeated_body_execution_reflects_fresh_state_each_iteration() {
        let (result, vm) = run("repeat 10 [forward 1]").unwrap();
        assert_eq!(result.output, "");
        assert_eq!(
            vm.turtles().state(TurtleId::new(0)).unwrap().position,
            Point::new(0.0, 10.0)
        );
    }

    #[test]
    fn run_and_runresult_on_same_list_use_distinct_cached_bytecode() {
        let (result, _) = run("make \"body [sum 2 3] \
             catch \"error [run :body] \
             print runresult :body")
        .unwrap();
        assert_eq!(result.output, "5\n");
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
    fn atari_word_and_number_helper_primitives_work() {
        let (result, _) = run("print rank [a b [c]]
             print ascii \"A
             print char 66
             print lowercase \"HeLLo
             print rev [a b c]
             print rev \"stressed
             print realwordp \"hello
             print realwordp 7
             print evenp 6
             print divisorp 3 12
             print factorial 5
             rerandom
             print ranpick [a b c]
             rerandom
             print ranpick [a b c]")
        .unwrap();
        let lines = result.output.lines().collect::<Vec<_>>();
        assert_eq!(lines[0], "2");
        assert_eq!(lines[1], "65");
        assert_eq!(lines[2], "B");
        assert_eq!(lines[3], "hello");
        assert_eq!(lines[4], "[c b a]");
        assert_eq!(lines[5], "desserts");
        assert_eq!(lines[6], "true");
        assert_eq!(lines[7], "false");
        assert_eq!(lines[8], "true");
        assert_eq!(lines[9], "true");
        assert_eq!(lines[10], "120");
        assert_eq!(lines[11], lines[12]);
        assert!(matches!(lines[11], "a" | "b" | "c"));
    }

    #[test]
    fn atari_outside_world_output_helpers_store_headless_requests() {
        let mut vm = Vm::new();
        vm.eval_source("setenv 1 2 3 4 toot 0 64 10 15 setcursor 5 7")
            .unwrap();

        assert_eq!(vm.sound_envelope(), [1, 2, 3, 4]);
        assert_eq!(vm.last_toot(), Some([0, 64, 10, 15]));
        assert_eq!(vm.text_cursor(), (5, 7));
    }

    #[test]
    fn atari_timeout_and_screen_mode_helpers_store_headless_requests() {
        let mut vm = Vm::new();
        assert_eq!(vm.text_screen_mode(), "splitscreen");
        vm.eval_source("timeout 90 textscreen splitscreen fullscreen fs ts ss")
            .unwrap();

        assert_eq!(vm.timeout_sixtieths(), 90);
        assert_eq!(vm.text_screen_mode(), "splitscreen");
    }

    #[test]
    fn atari_input_helpers_report_mocked_device_state() {
        let mut vm = Vm::new();
        vm.push_keypress('a');
        vm.set_joystick_state(1, 6, true);
        vm.set_paddle_state(2, 99, true);

        let result = vm
            .eval_source("print keyp print joy 1 print joyb 1 print paddle 2 print paddleb 2")
            .unwrap();
        assert_eq!(result.output, "true\n6\ntrue\n99\ntrue\n");
    }

    #[test]
    fn over_reporter_detects_pen_contact() {
        let (result, _) =
            run("tell 0 setpc 3 fd 10 pu setxy 20 20 tell 1 setxy 0 5 print over 3").unwrap();
        assert_eq!(result.output, "true\n");
    }

    #[test]
    fn label_emits_a_headless_label_event() {
        let (_, vm) = run("setxy 10 20 setpc 5 setlabelheight 24 label [hello logo]").unwrap();
        let label = vm.turtles().events().iter().find_map(|event| match event {
            TurtleEvent::Label {
                at,
                text,
                color,
                height,
            } => Some((*at, text.clone(), *color, *height)),
            _ => None,
        });
        assert_eq!(
            label,
            Some((Point::new(10.0, 20.0), "[hello logo]".to_string(), 5, 24.0))
        );
    }

    #[test]
    fn fill_emits_a_headless_fill_event() {
        let (_, vm) = run("setxy -5 8 setpc 7 fill").unwrap();
        let fill = vm.turtles().events().iter().find_map(|event| match event {
            TurtleEvent::Fill { at, color } => Some((*at, *color)),
            _ => None,
        });
        assert_eq!(fill, Some((Point::new(-5.0, 8.0), 7)));
    }

    #[test]
    fn when_over_fires_during_dynaturtle_tick() {
        let mut vm = Vm::new();
        vm.eval_source(
            "tell 0 setpc 3 fd 10 pu setxy 20 20 tell 1 setxy 0 5 when [over 3] [print \"hit]",
        )
        .unwrap();
        vm.clear_output();
        vm.dynaturtle_tick(0.0).unwrap();
        assert_eq!(vm.output(), "hit\n");
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
             make \"bar [a b]
             pprop \"animal \"legs 4",
        )
        .unwrap();
        let result = vm
            .eval_source("pots pops pons popls poall po [alpha] .primitives")
            .unwrap();
        assert!(result.output.contains("[to alpha :x]"));
        assert!(result.output.contains("[to beta :y]"));
        assert!(result.output.contains("[print :x]"));
        assert!(result.output.contains("[output sum :y 1]"));
        assert!(result.output.contains("animal [legs 4]"));
        assert!(result.output.contains("bar [a b]"));
        assert!(result.output.contains("foo 7"));
        assert!(result.output.contains("sum + difference"));
        assert!(!result.output.contains("__whileloop"));
    }

    #[test]
    fn bury_and_unbury_hide_workspace_entries_from_listings() {
        let mut vm = Vm::new();
        vm.eval_source(
            "to alpha :x
             output :x
             end
             make \"foo 7
             pprop \"animal \"legs 4
             bury [alpha foo animal]",
        )
        .unwrap();

        let result = vm.eval_source("pops pons popls poall").unwrap();
        assert!(!result.output.contains("alpha"));
        assert!(!result.output.contains("foo 7"));
        assert!(!result.output.contains("animal [legs 4]"));

        vm.clear_output();
        let result = vm
            .eval_source("print buriedp \"alpha print buriedp \"foo print buriedp \"animal")
            .unwrap();
        assert_eq!(result.output, "true\ntrue\ntrue\n");

        vm.clear_output();
        vm.eval_source("unbury [alpha foo animal]").unwrap();
        let result = vm.eval_source("pops pons popls").unwrap();
        assert!(result.output.contains("alpha"));
        assert!(result.output.contains("foo 7"));
        assert!(result.output.contains("animal [legs 4]"));
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
        vm.eval_source("pprop \"animal \"legs 4 erpl [animal]")
            .unwrap();
        vm.clear_output();
        let result = vm.eval_source("print plist \"animal").unwrap();
        assert_eq!(result.output, "[]\n");

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
    fn wait_zero_is_noop_and_error_without_caught_failure_is_empty_list() {
        let (result, _) = run("wait 0 print error").unwrap();
        assert_eq!(result.output, "[]\n");
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
    fn error_primitive_reports_empty_list_after_uncaught_failure() {
        let mut vm = Vm::new();
        let error = vm.eval_source("print").unwrap_err();
        assert!(error.message.starts_with("not enough inputs to print"));
        let PrimitiveResult::Value(Value::List(list)) = vm.error(vec![]).unwrap() else {
            panic!("ERROR should output a list");
        };
        assert!(list.is_empty());
    }

    #[test]
    fn catch_error_populates_structured_error_list_and_consumes_it() {
        let mut vm = Vm::new();
        let result = vm.eval_source("catch \"error [print]").unwrap();
        assert_eq!(result.output, "");

        let PrimitiveResult::Value(Value::List(list)) = vm.error(vec![]).unwrap() else {
            panic!("ERROR should output a list");
        };
        assert_eq!(list.item(1).unwrap().show(vm.interner()), "6");
        assert_eq!(
            list.item(2).unwrap().show(vm.interner()),
            "not enough inputs to print"
        );
        assert_eq!(list.item(3).unwrap().show(vm.interner()), "");

        let PrimitiveResult::Value(Value::List(empty)) = vm.error(vec![]).unwrap() else {
            panic!("ERROR should output a list");
        };
        assert!(empty.is_empty());
    }

    #[test]
    fn catch_error_records_unknown_procedure_code_and_context() {
        let mut vm = Vm::new();
        let symbol = vm.interner_mut().intern("mystery");
        let error = vm.call(symbol, vec![], false).unwrap_err();
        vm.record_caught_error(&error);

        let PrimitiveResult::Value(Value::List(list)) = vm.error(vec![]).unwrap() else {
            panic!("ERROR should output a list");
        };
        assert_eq!(list.item(1).unwrap().show(vm.interner()), "13");
        assert_eq!(
            list.item(2).unwrap().show(vm.interner()),
            "I don't know how to mystery"
        );
        assert_eq!(list.item(3).unwrap().show(vm.interner()), "mystery");
    }

    #[test]
    fn pause_continue_resumes_with_mutated_environment() {
        let mut vm = Vm::new();
        vm.push_pause_input("make \"x sum :x 1");
        vm.push_pause_input("continue");
        let result = vm
            .eval_source("make \"x 5 pause print :x")
            .expect("pause should resume after CONTINUE");
        assert_eq!(result.output, "6\n");
    }

    #[test]
    fn continue_outside_pause_is_an_error() {
        let mut vm = Vm::new();
        let error = vm.eval_source("continue").unwrap_err();
        assert_eq!(error.message, "CONTINUE can only be used inside PAUSE");
    }

    #[test]
    fn pause_can_output_from_the_enclosing_procedure() {
        let mut vm = Vm::new();
        vm.push_pause_input("output 99");
        let result = vm
            .eval_source(
                "to paused.output
                 pause
                 output 0
                 end
                 print paused.output",
            )
            .expect("OUTPUT entered during PAUSE should resume the procedure with a value");
        assert_eq!(result.output, "99\n");
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
        assert_eq!(error.message, "noop didn't output to print");
    }

    #[test]
    fn turtle_motion_primitives_update_state_and_record_lines() {
        let (result, vm) = run("fd 100 rt 90 fd 50 print pos print heading").unwrap();
        assert_eq!(result.output, "[50 100]\n90\n");
        let state = vm.turtles().state(TurtleId::new(0)).unwrap();
        assert_eq!(state.position, Point::new(50.0, 100.0));
        assert_eq!(state.heading, 90.0);
        let line_count = vm
            .turtles()
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
        let state = vm.turtles().state(TurtleId::new(0)).unwrap();
        assert!(state.pen_down);
        assert!(state.visible);
        assert_eq!(state.pen_color, 3);
        assert_eq!(state.pen_size, 4.0);
        let line_count = vm
            .turtles()
            .events()
            .iter()
            .filter(|event| matches!(event, TurtleEvent::Line { .. }))
            .count();
        assert_eq!(line_count, 1);
    }

    #[test]
    fn setpos_setxy_home_and_clearscreen() {
        let (_, mut vm) = run("setpos [10 20] setxy 30 40 home cs").unwrap();
        let state = vm.turtles().state(TurtleId::new(0)).unwrap();
        assert_eq!(state.position, Point::new(0.0, 0.0));
        assert_eq!(state.heading, 0.0);
        assert!(vm
            .turtles()
            .events()
            .iter()
            .any(|event| matches!(event, TurtleEvent::Clear)));
        vm.turtles_mut().clear_events();
        assert!(vm.turtles().events().is_empty());
    }

    #[test]
    fn atari_turtle_state_primitives_setx_sety_and_shownp() {
        let (result, vm) =
            run("ht print shownp setx 25 sety -10 st print shownp print pos").unwrap();
        assert_eq!(result.output, "false\ntrue\n[25 -10]\n");
        let state = vm.turtles().state(TurtleId::new(0)).unwrap();
        assert_eq!(state.position, Point::new(25.0, -10.0));
        assert!(state.visible);
    }

    #[test]
    fn adjacent_negative_literals_survive_instruction_list_execution() {
        let (_, vm) = run("run [setxy -20 -30]").unwrap();
        assert_eq!(
            vm.turtles().state(TurtleId::new(0)).unwrap().position,
            Point::new(-20.0, -30.0)
        );
    }

    #[test]
    fn adjacent_negative_literals_survive_direct_top_level_parsing() {
        let (_, vm) = run("setxy -20 -30").unwrap();
        assert_eq!(
            vm.turtles().state(TurtleId::new(0)).unwrap().position,
            Point::new(-20.0, -30.0)
        );
    }

    #[test]
    fn init_turtle_resets_display_to_visible_default_turtle() {
        let (_, vm) = run("ht setxy 10 20 init.turtle").unwrap();
        let state = vm.turtles().state(TurtleId::new(0)).unwrap();
        assert_eq!(state.position, Point::new(0.0, 0.0));
        assert_eq!(state.heading, 0.0);
        assert!(state.visible);
        assert!(vm
            .turtles()
            .events()
            .iter()
            .any(|event| matches!(event, TurtleEvent::Clear)));
    }

    #[test]
    fn tell_ask_each_who_select_and_restore_active_turtles() {
        let (result, _) = run("tell [1 2] ask 3 [print who] print who").unwrap();
        assert_eq!(result.output, "[3]\n[1 2]\n");
    }

    #[test]
    fn each_runs_body_once_per_active_turtle() {
        let (result, _) = run("tell [1 2 3] each [print who]").unwrap();
        assert_eq!(result.output, "[1]\n[2]\n[3]\n");
    }

    #[test]
    fn tell_selects_which_turtle_classic_motion_primitives_move() {
        let (_, vm) = run("tell 3 forward 10").unwrap();
        assert_eq!(
            vm.turtles().state(TurtleId::new(3)).unwrap().position,
            Point::new(0.0, 10.0)
        );
        assert_eq!(
            vm.turtles().state(TurtleId::new(0)).unwrap().position,
            Point::new(0.0, 0.0)
        );
    }

    #[test]
    fn tell_many_broadcasts_classic_motion_and_pen_primitives() {
        let (_, vm) = run("tell [1 2] right 90 forward 5 penup").unwrap();
        for id in [TurtleId::new(1), TurtleId::new(2)] {
            let state = vm.turtles().state(id).unwrap();
            assert_eq!(state.heading, 90.0);
            assert!((state.position.x - 5.0).abs() < 1e-9);
            assert!(state.position.y.abs() < 1e-9);
            assert!(!state.pen_down);
        }
        let untouched = vm.turtles().state(TurtleId::new(0)).unwrap();
        assert_eq!(untouched.position, Point::new(0.0, 0.0));
        assert!(untouched.pen_down);
    }

    #[test]
    fn ask_scopes_classic_motion_primitives_to_a_single_turtle() {
        let (_, vm) = run("tell [0 1] ask 5 [forward 20]").unwrap();
        assert_eq!(
            vm.turtles().state(TurtleId::new(5)).unwrap().position,
            Point::new(0.0, 20.0)
        );
        assert_eq!(
            vm.turtles().state(TurtleId::new(0)).unwrap().position,
            Point::new(0.0, 0.0)
        );
        assert_eq!(
            vm.turtles().state(TurtleId::new(1)).unwrap().position,
            Point::new(0.0, 0.0)
        );
    }

    #[test]
    fn setspeed_projects_velocity_along_turtle_stores_own_heading() {
        let (_, vm) = run("tell 0 setspeed 20").unwrap();
        assert_eq!(
            vm.turtles().velocity(TurtleId::new(0)),
            Some(Point::new(0.0, 20.0))
        );
    }

    #[test]
    fn setvelocity_writes_raw_velocity_to_the_turtle_store() {
        let (_, vm) = run("tell 0 setvelocity 3 4").unwrap();
        assert_eq!(
            vm.turtles().velocity(TurtleId::new(0)),
            Some(Point::new(3.0, 4.0))
        );
    }

    #[test]
    fn setshape_records_shape_name_and_collision_radius() {
        let (_, vm) = run("tell 1 setshape \"ship 20").unwrap();
        assert_eq!(vm.turtles().shape(TurtleId::new(1)), Some("ship"));
        assert_eq!(vm.turtles().collision_radius(TurtleId::new(1)), Some(20.0));
    }

    #[test]
    fn touching_reports_whether_two_turtles_overlap() {
        let (_, mut vm) = run("init.turtle").unwrap();
        vm.turtles_mut()
            .set_position(TurtleId::new(0), Point::new(0.0, 0.0));
        vm.turtles_mut()
            .set_position(TurtleId::new(1), Point::new(5.0, 0.0));
        vm.turtles_mut()
            .set_position(TurtleId::new(2), Point::new(500.0, 500.0));
        let result = vm.eval_source("print touching 0 1").unwrap();
        assert_eq!(result.output, "true\n");
        vm.clear_output();
        let result = vm.eval_source("print touching 0 2").unwrap();
        assert_eq!(result.output, "false\n");
    }

    #[test]
    fn when_registers_a_touching_demon_condition_and_body() {
        let (_, vm) = run("when [touching 0 1] [print \"boom]").unwrap();
        assert_eq!(vm.demons().demons().len(), 1);
        assert_eq!(
            *vm.demons().demons()[0].condition(),
            DemonCondition::Touching(TurtleId::new(0), TurtleId::new(1))
        );
    }

    #[test]
    fn dynaturtle_tick_integrates_detects_collisions_and_fires_demon_bodies() {
        let mut vm = Vm::new();
        vm.eval_source("when [touching 0 1] [print \"collided]")
            .unwrap();
        vm.turtles_mut()
            .set_position(TurtleId::new(0), Point::new(0.0, 0.0));
        vm.turtles_mut()
            .set_position(TurtleId::new(1), Point::new(20.0, 0.0));
        vm.turtles_mut()
            .set_velocity(TurtleId::new(1), Point::new(-30.0, 0.0));
        vm.clear_output();

        vm.dynaturtle_tick(0.5).unwrap();

        assert_eq!(
            vm.turtles().positions()[TurtleId::new(1).index()],
            Point::new(5.0, 0.0)
        );
        assert_eq!(vm.output(), "collided\n");
    }

    #[test]
    fn toot_records_last_sound_event() {
        let (_, vm) = run("toot 0 64 10 15").unwrap();
        assert_eq!(vm.last_toot(), Some([0, 64, 10, 15]));
    }
}
