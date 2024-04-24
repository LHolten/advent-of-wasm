use serde::Deserialize;

use std::{collections::HashMap, fs, path::PathBuf};

use fehler::throws;
use wasmtime::{Engine, Module};

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

#[cfg(test)]
mod tests {

    // use wasmtime::{Config, Engine};

    // use crate::solution::Solution;

    // use super::ProblemDir;

    // #[test]
    // fn gen_instance() -> anyhow::Result<()> {
    //     let dir = ProblemDir::new()?;
    //     let engine = Engine::default();
    //     let problem_hash = dir.mapping["parse"];
    //     let problem = dir.problems[&problem_hash].generate(&engine, 30)?;
    //     assert_eq!(&*problem.input, b"30");

    //     let solution = Solution {
    //         hash: "bDHNXb6S_4Y".parse().unwrap(),
    //     };
    //     let engine = Engine::new(Config::new().consume_fuel(true))?;
    //     let res = solution.run(&engine, &problem.input, 10000);
    //     assert_eq!(res.unwrap().answer, 30);

    //     Ok(())
    // }
}
