use anyhow::Context;
use rand::Rng;
use serde::Deserialize;

use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use fehler::throws;
use wasmtime::{Caller, Engine, Func, Linker, Module, Store, TypedFunc};

#[derive(Deserialize)]
pub struct Task {
    pub name: String,
    generator: std::path::PathBuf,
    pub fuel: u64,
}

#[derive(Deserialize)]
pub struct TaskDir {
    pub tasks: Vec<Task>,
}

impl TaskDir {
    #[throws(anyhow::Error)]
    pub fn new() -> Self {
        let content = fs::read_to_string("tasks.toml")?;
        toml::from_str(&content)?
    }
}

impl Task {
    #[throws(anyhow::Error)]
    pub fn load_module(&self, engine: &Engine) -> Module {
        if let Ok(module) = self.load_cached(engine) {
            return module;
        }
        let module = Module::from_file(engine, &self.generator)?;
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
        let mut cache_path = self.generator.clone();
        let true = cache_path.set_extension("compiled") else {
            anyhow::bail!("can not change extensions of path {}", cache_path.display())
        };
        cache_path
    }
}

pub struct TaskInstance {
    data: Box<[u8]>,
}

impl Task {
    #[throws(anyhow::Error)]
    pub fn generate<R: Rng>(&self, engine: &Engine, rng: R) -> TaskInstance {
        let module = self.load_module(engine)?;
        let mut store = Store::new(&engine, rng);

        let mut linker = Linker::new(&engine);
        let host_rand = Func::wrap(&mut store, |mut caller: Caller<'_, R>| {
            caller.data_mut().gen::<i64>()
        });
        linker.define("env", "rand", host_rand);

        // first instantiate, this calls optional start
        let instance = linker.instantiate(&mut store, &module)?;
        // call the generator
        let func: TypedFunc<_, (i32, i32)> = instance.get_typed_func(&mut store, "generate")?;
        let (offset, length) = func.call(&mut store, ())?;

        // read the generated instance from wasm
        let mut data = vec![0; length as usize].into_boxed_slice();
        let memory = instance
            .get_memory(&mut store, "memory")
            .context("memory was not defined")?;
        memory.read(&store, offset as usize, &mut *data)?;

        TaskInstance { data }
    }
}
