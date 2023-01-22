use anyhow::Context;
use serde::Deserialize;

use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use fehler::throws;
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};

use crate::hash::FileHash;

#[derive(Deserialize)]
pub struct ModulePath(pub std::path::PathBuf);

#[derive(Deserialize)]
pub struct Problem {
    pub file_name: ModulePath,
    pub leaderboard_instances: u32, // this is how many of the oldest instances need to be ran
    pub fuel_limit: u64,
}

#[derive(Deserialize)]
pub struct ProblemDir {
    pub problems: HashMap<FileHash, Problem>,
    pub mapping: HashMap<String, FileHash>,
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
    fn load_cached(&self, engine: &Engine) -> Module {
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
    pub fn hash(&self) -> FileHash {
        let buf = fs::read(&self.0)?;
        FileHash::new(buf)
    }
}

pub struct TaskInstance {
    pub input: Box<[u8]>,
    pub answer: i64,
}

impl Problem {
    #[throws(anyhow::Error)]
    pub fn generate(&self, engine: &Engine, seed: u64) -> TaskInstance {
        let module = self.file_name.load(engine)?;
        let mut store = Store::new(engine, ());

        // first instantiate, this calls optional start
        let instance = Linker::new(engine).instantiate(&mut store, &module)?;
        // call the generator
        let func: TypedFunc<_, (i32, i32)> = instance.get_typed_func(&mut store, "generate")?;
        let (offset, length) = func.call(&mut store, seed)?;

        // read the generated instance from wasm
        let mut input = vec![0; length as usize].into_boxed_slice();
        let memory = instance
            .get_memory(&mut store, "memory")
            .context("memory was not defined")?;
        memory.read(&store, offset as usize, &mut input)?;

        // let solve: TypedFunc<_, (i32, i32)> = instance.get_typed_func(&mut store, "solution")?;
        // let (offset, length) = solve.call(&mut store, (offset, length))?;

        // let mut output = vec![0; length as usize].into_boxed_slice();
        // let memory = instance
        //     .get_memory(&mut store, "memory")
        //     .context("memory was not defined")?;
        // memory.read(&store, offset as usize, &mut output)?;

        TaskInstance { input, answer: 0 }
    }
}

#[cfg(test)]
mod tests {

    use wasmtime::Engine;

    use super::ProblemDir;

    #[test]
    fn gen_instance() -> anyhow::Result<()> {
        let dir = ProblemDir::new()?;
        let engine = Engine::default();
        let problem_hash = dir.mapping["parse"];
        let problem = dir.problems[&problem_hash].generate(&engine, 30)?;
        assert_eq!(&*problem.input, b"30");
        // assert_eq!(&*problem.output, &u64::to_be_bytes(30)[..]);

        Ok(())
    }
}
