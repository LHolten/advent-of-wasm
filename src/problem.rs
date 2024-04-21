use anyhow::Context;
use serde::Deserialize;

use std::{collections::HashMap, fs, path::PathBuf};

use fehler::throws;
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};

use crate::hash::FileHash;

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

#[derive(Deserialize)]
pub struct ModulePath(pub std::path::PathBuf);

impl ModulePath {
    #[throws(anyhow::Error)]
    pub fn load(&self, engine: &Engine) -> Module {
        if let Ok(module) = self.load_cached(engine) {
            return module;
        }
        let module = Module::from_file(engine, &self.0)?;
        fs::write(self.cache_path()?, module.serialize()?)?;
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

    #[throws(anyhow::Error)]
    pub fn file_len(&self) -> usize {
        let buf = fs::read(&self.0)?;
        buf.len()
    }
}

pub struct TaskInstance {
    pub input: Box<[u8]>,
    pub answer: i64,
}

impl Problem {
    #[throws(anyhow::Error)]
    pub fn generate(&self, engine: &Engine, seed: i64) -> TaskInstance {
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

        let solution: TypedFunc<_, i64> = instance.get_typed_func(&mut store, "solution")?;
        let answer = solution.call(&mut store, (offset, length))?;

        TaskInstance { input, answer }
    }
}

#[cfg(test)]
mod tests {

    use wasmtime::{Config, Engine};

    use crate::solution::Solution;

    use super::ProblemDir;

    #[test]
    fn gen_instance() -> anyhow::Result<()> {
        let dir = ProblemDir::new()?;
        let engine = Engine::default();
        let problem_hash = dir.mapping["parse"];
        let problem = dir.problems[&problem_hash].generate(&engine, 30)?;
        assert_eq!(&*problem.input, b"30");

        let solution = Solution {
            hash: "bDHNXb6S_4Y".parse().unwrap(),
        };
        let engine = Engine::new(Config::new().consume_fuel(true))?;
        let res = solution.run(&engine, &problem.input, 10000);
        assert_eq!(res.unwrap().answer, 30);

        Ok(())
    }
}
