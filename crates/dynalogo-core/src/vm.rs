//! Stack VM for DynaLOGO bytecode.
//!
//! This is the initial v0.1 executor. It establishes the pieces that later
//! procedure support will build on: a shared interner, dynamic-scope frame
//! stack, primitive dispatch, bytecode stack execution, and `OUTPUT`/`STOP`
//! control signals.

use std::collections::HashMap;
use std::fmt;

use crate::bytecode::{Chunk, Instruction};
use crate::lexer::InfixOp;
use crate::value::{Interner, LogoNumber, Symbol, Value};

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

#[derive(Debug, Default, Clone)]
pub struct Vm {
    interner: Interner,
    env: Environment,
    output: String,
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

    fn call(&mut self, callee: Symbol, args: Vec<Value>) -> Result<PrimitiveResult, VmError> {
        let name = self.interner.canonical_spelling(callee).to_string();
        match name.as_str() {
            "sum" | "+" => self.number_binop(args, |a, b| a + b),
            "difference" | "-" => self.number_binop(args, |a, b| a - b),
            "product" | "*" => self.number_binop(args, |a, b| a * b),
            "quotient" | "/" => self.number_binop(args, |a, b| a / b),
            "remainder" => self.number_binop(args, |a, b| a % b),
            "equalp" | "equal?" => self.equalp(args),
            "print" | "pr" => self.print(args),
            "show" => self.show(args),
            "type" => self.r#type(args),
            "make" | "name" => self.make(args),
            "thing" => self.thing(args),
            "output" | "op" => self.output_control(args),
            "stop" => {
                expect_arity(&name, &args, 0).map(|()| PrimitiveResult::Control(ControlFlow::Stop))
            }
            _ => Err(VmError::new(format!("I don't know how to {name}"))),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Compiler;
    use crate::parser::{parse_source, ArityTable};

    fn run(source: &str) -> Result<(RunResult, Vm), VmError> {
        let mut vm = Vm::new();
        let program = parse_source(source, vm.interner_mut(), &ArityTable::default())
            .map_err(|error| VmError::new(error.to_string()))?;
        let chunk = Compiler::new()
            .compile_program(&program)
            .map_err(|error| VmError::new(error.to_string()))?;
        let result = vm.run(&chunk)?;
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
}
