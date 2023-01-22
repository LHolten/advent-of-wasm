use std::path::PathBuf;

use anyhow::Context;
use fehler::throws;
use wasmtime::{Engine, Linker, Store, TypedFunc};

use crate::{hash::FileHash, problem::ModulePath};

pub struct Solution {
    pub hash: FileHash,
}

impl Solution {
    #[throws(anyhow::Error)]
    pub fn run_submission(&self, engine: &Engine, data: &[u8], fuel: u64) -> i64 {
        let mut path = PathBuf::from("submission");
        path.push(format!("{}.wasm", self.hash.to_string()));

        let module = ModulePath(path).load(engine)?;
        let mut store = Store::new(engine, ());

        // first instantiate, this calls optional start
        // add some fuel here so the program can run
        store.add_fuel(fuel).unwrap();
        let instance = Linker::new(engine).instantiate(&mut store, &module)?;

        // we need to get the base of the wasm heap so we don't interfere with stack space.
        let heap_base = instance
            .get_global(&mut store, "__heap_base")
            .context("expected a global called '__heap_base'")?;
        let heap_base = heap_base
            .get(&mut store)
            .i32()
            .context("expected global '__heap_base' to be an i32")?;

        // now we can write the actual input
        let memory = instance
            .get_memory(&mut store, "memory")
            .context("expected a memory called `memory`")?;
        memory.write(&mut store, heap_base as usize, data)?;
        let input = data.len() as i32;

        // call the actual solve function
        let func: TypedFunc<_, i64> = instance.get_typed_func(&mut store, "solve")?;
        func.call(&mut store, input)?
    }
}
