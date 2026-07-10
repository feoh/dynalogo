//! Bytecode compiler and chunk cache.
//!
//! The VM is intentionally not a tree-walker. Parser output is lowered into a
//! compact stack bytecode stream, and compiled chunks can be cached by procedure
//! or by instruction-list identity. Later `TO`, `DEFINE`, and `ERASE` support
//! will invalidate these cache keys when definitions change.

use std::collections::HashMap;
use std::fmt;

use crate::lexer::InfixOp;
use crate::parser::{Expr, Program};
use crate::value::{List, Symbol, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Push(Value),
    LoadThing(Symbol),
    Call { callee: Symbol, argc: usize },
    Infix(InfixOp),
    Halt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    instructions: Vec<Instruction>,
}

impl Chunk {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self { instructions }
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn into_instructions(self) -> Vec<Instruction> {
        self.instructions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub message: String,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug, Default, Clone)]
pub struct Compiler;

impl Compiler {
    pub fn new() -> Self {
        Self
    }

    pub fn compile_program(&self, program: &Program) -> Result<Chunk, CompileError> {
        let mut instructions = Vec::new();
        for expression in program.expressions() {
            self.compile_expr(expression, &mut instructions)?;
        }
        instructions.push(Instruction::Halt);
        Ok(Chunk::new(instructions))
    }

    pub fn compile_expr(
        &self,
        expression: &Expr,
        instructions: &mut Vec<Instruction>,
    ) -> Result<(), CompileError> {
        match expression {
            Expr::Literal(value) => instructions.push(Instruction::Push(value.clone())),
            Expr::Thing(symbol) => instructions.push(Instruction::LoadThing(*symbol)),
            Expr::Call { callee, args, .. } => {
                for arg in args {
                    self.compile_expr(arg, instructions)?;
                }
                instructions.push(Instruction::Call {
                    callee: *callee,
                    argc: args.len(),
                });
            }
            Expr::Infix { op, left, right } => {
                self.compile_expr(left, instructions)?;
                self.compile_expr(right, instructions)?;
                instructions.push(Instruction::Infix(*op));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkKey {
    Procedure(Symbol),
    InstructionList(usize),
    SourceHash(u64),
}

impl ChunkKey {
    pub fn for_list(list: &List) -> Option<Self> {
        list.pointer_identity().map(Self::InstructionList)
    }
}

#[derive(Debug, Clone)]
struct CachedChunk {
    chunk: Chunk,
    generation: u64,
}

#[derive(Debug, Default, Clone)]
pub struct ChunkCache {
    chunks: HashMap<ChunkKey, CachedChunk>,
    generation: u64,
}

impl ChunkCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn get(&self, key: ChunkKey) -> Option<&Chunk> {
        self.chunks.get(&key).map(|cached| &cached.chunk)
    }

    pub fn insert(&mut self, key: ChunkKey, chunk: Chunk) {
        self.chunks.insert(
            key,
            CachedChunk {
                chunk,
                generation: self.generation,
            },
        );
    }

    pub fn get_or_insert_with<E>(
        &mut self,
        key: ChunkKey,
        compile: impl FnOnce() -> Result<Chunk, E>,
    ) -> Result<&Chunk, E> {
        if !self.chunks.contains_key(&key) {
            let chunk = compile()?;
            self.insert(key, chunk);
        }
        Ok(self
            .chunks
            .get(&key)
            .expect("chunk was just inserted")
            .chunk())
    }

    pub fn invalidate(&mut self, key: ChunkKey) -> bool {
        let removed = self.chunks.remove(&key).is_some();
        if removed {
            self.bump_generation();
        }
        removed
    }

    pub fn invalidate_procedure(&mut self, procedure: Symbol) -> bool {
        self.invalidate(ChunkKey::Procedure(procedure))
    }

    pub fn clear(&mut self) {
        if !self.chunks.is_empty() {
            self.chunks.clear();
            self.bump_generation();
        }
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
}

impl CachedChunk {
    fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    #[allow(dead_code)]
    fn generation(&self) -> u64 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_source, ArityTable};
    use crate::value::Interner;

    fn compile(source: &str) -> (Chunk, Interner) {
        let mut interner = Interner::new();
        let program = parse_source(source, &mut interner, &ArityTable::default()).unwrap();
        let chunk = Compiler::new().compile_program(&program).unwrap();
        (chunk, interner)
    }

    #[test]
    fn compiles_prefix_call_stack_order() {
        let (chunk, mut interner) = compile("print sum 1 2");
        let instructions = chunk.instructions();
        assert_eq!(instructions[0], Instruction::Push(Value::number(1.0)));
        assert_eq!(instructions[1], Instruction::Push(Value::number(2.0)));

        let sum = interner.intern("sum");
        let print = interner.intern("print");
        assert_eq!(
            instructions[2],
            Instruction::Call {
                callee: sum,
                argc: 2
            }
        );
        assert_eq!(
            instructions[3],
            Instruction::Call {
                callee: print,
                argc: 1
            }
        );
        assert_eq!(instructions[4], Instruction::Halt);
    }

    #[test]
    fn compiles_infix_after_operands() {
        let (chunk, _) = compile("2+3*4");
        assert_eq!(
            chunk.instructions(),
            &[
                Instruction::Push(Value::number(2.0)),
                Instruction::Push(Value::number(3.0)),
                Instruction::Push(Value::number(4.0)),
                Instruction::Infix(InfixOp::Star),
                Instruction::Infix(InfixOp::Plus),
                Instruction::Halt,
            ]
        );
    }

    #[test]
    fn compiles_colon_words_as_loadthing() {
        let (chunk, mut interner) = compile("print :x");
        let x = interner.intern("x");
        assert_eq!(chunk.instructions()[0], Instruction::LoadThing(x));
    }

    #[test]
    fn chunk_cache_get_or_insert_compiles_once() {
        let mut interner = Interner::new();
        let proc = interner.intern("foo");
        let mut cache = ChunkCache::new();
        let mut count = 0;

        let first = cache
            .get_or_insert_with(ChunkKey::Procedure(proc), || {
                count += 1;
                Ok::<_, CompileError>(Chunk::new(vec![Instruction::Halt]))
            })
            .unwrap()
            .instructions()
            .len();
        let second = cache
            .get_or_insert_with(ChunkKey::Procedure(proc), || {
                count += 1;
                Ok::<_, CompileError>(Chunk::new(vec![]))
            })
            .unwrap()
            .instructions()
            .len();

        assert_eq!(first, 1);
        assert_eq!(second, 1);
        assert_eq!(count, 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn cache_invalidation_bumps_generation() {
        let mut interner = Interner::new();
        let proc = interner.intern("foo");
        let mut cache = ChunkCache::new();
        cache.insert(
            ChunkKey::Procedure(proc),
            Chunk::new(vec![Instruction::Halt]),
        );
        assert_eq!(cache.generation(), 0);
        assert!(cache.invalidate_procedure(proc));
        assert_eq!(cache.generation(), 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn list_identity_key_tracks_nonempty_lists() {
        let list = List::from_values([Value::number(1.0)]);
        let empty = List::empty();
        assert!(ChunkKey::for_list(&list).is_some());
        assert!(ChunkKey::for_list(&empty).is_none());
    }
}
