use anyhow::Context;
use rand::Rng;
use serde::Deserialize;

use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use fehler::throws;
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};

use crate::hash::Hash;

#[derive(Deserialize)]
pub struct ModulePath(pub std::path::PathBuf);

#[derive(Deserialize)]
pub struct Problem {
    pub file_name: ModulePath,
    pub file_hash: String,
}

#[derive(Deserialize)]
pub struct ProblemDir {
    pub problems: HashMap<String, Problem>,
}

impl ProblemDir {
    #[throws(anyhow::Error)]
    pub fn new() -> Self {
        let content = fs::read_to_string("config/problem.toml")?;
        toml::from_str(&content)?
    }
}

impl ModulePath {
    #[throws(anyhow::Error)]
    pub fn load(&self, engine: &Engine) -> Module {
        if let Ok(module) = self.load_cached(engine) {
            return module;
        }
        let module = Module::from_file(engine, &self.0)?;
        let buf = module.serialize()?;
        let mut file = File::create(self.cache_path()?)?;
        file.write_all(&buf)?;
        module
    }

    #[throws(anyhow::Error)]
    pub fn load_cached(&self, engine: &Engine) -> Module {
        let cache_path = self.cache_path()?;
        unsafe { Module::deserialize_file(engine, cache_path)? }
    }

    #[throws(anyhow::Error)]
    fn cache_path(&self) -> PathBuf {
        let mut cache_path = self.0.clone();
        let true = cache_path.set_extension("compiled") else {
            anyhow::bail!("can not change extensions of path {}", cache_path.display())
        };
        cache_path
    }

    #[throws(anyhow::Error)]
    pub fn hash(&self) -> Hash {
        let buf = fs::read(&self.0)?;
        Hash::new(buf)
    }
}

pub struct TaskInstance {
    data: Box<[u8]>,
}

impl Problem {
    #[throws(anyhow::Error)]
    pub fn generate<R: Rng>(&self, engine: &Engine, seed: u64) -> TaskInstance {
        let module = self.file_name.load(engine)?;
        let mut store = Store::new(engine, ());

        // first instantiate, this calls optional start
        let instance = Linker::new(engine).instantiate(&mut store, &module)?;
        // call the generator
        let func: TypedFunc<_, (i32, i32)> = instance.get_typed_func(&mut store, "generate")?;
        let (offset, length) = func.call(&mut store, seed)?;

        // read the generated instance from wasm
        let mut data = vec![0; length as usize].into_boxed_slice();
        let memory = instance
            .get_memory(&mut store, "memory")
            .context("memory was not defined")?;
        memory.read(&store, offset as usize, &mut data)?;

        TaskInstance { data }
    }
}
