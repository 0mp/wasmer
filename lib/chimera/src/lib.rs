#![feature(stdsimd)]

use wasmer_runtime_core::{
    backend::{Compiler, Token},
    cache::{Artifact, Error as CacheError},
    error::CompileError,
    module::ModuleInner,
};

mod clif;
mod compile;
mod pool;
mod state;
mod thunks;

pub trait TieredCompiler {}

pub struct Chimera {}
