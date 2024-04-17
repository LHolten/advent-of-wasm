use anyhow::Context;
use fehler::throws;
use wasmtime::{
    Engine, FuncType, GlobalType, Linker, Module, Mutability, Store, TypedFunc, ValType,
};

use crate::{hash::FileHash, problem::ModulePath};

pub struct Solution {
    pub hash: FileHash,
}

#[derive(Debug)]
pub struct RunResult {
    pub fuel_used: u64,
    pub answer: Option<i64>,
}

impl Solution {
    pub fn run(&self, engine: &Engine, data: &[u8], fuel: u64) -> RunResult {
        let path = format!("solution/{}.wasm", &self.hash);
        let module = ModulePath(path.into()).load(engine).unwrap();
        // first instantiate, this calls optional start
        // add some fuel here so the program can run
        let mut store = Store::new(engine, ());
        store.add_fuel(fuel).unwrap();
        let instance = Linker::new(engine)
            .instantiate(&mut store, &module)
            .unwrap();

        // we need to get the base of the wasm heap so we don't interfere with stack space.
        let heap_base = instance.get_global(&mut store, "__heap_base").unwrap();
        let heap_base = heap_base.get(&mut store).i32().unwrap();

        // now we can write the actual input
        let memory = instance.get_memory(&mut store, "memory").unwrap();

        let func: TypedFunc<i32, i64> = instance.get_typed_func(&mut store, "solve").unwrap();

        memory.grow(&mut store, 1).unwrap();
        memory.write(&mut store, heap_base as usize, data).unwrap();

        // call the actual solve function
        let answer = func
            .call(&mut store, data.len() as i32)
            .inspect_err(|e| println!("ERROR: {} {e}", self.hash))
            .ok();

        RunResult {
            fuel_used: store.fuel_consumed().unwrap(),
            answer,
        }
    }
}

#[throws(anyhow::Error)]
pub fn verify_wasm(buf: &[u8]) {
    let engine = Engine::default();
    let module = Module::from_binary(&engine, buf)?;

    let ftype = module
        .get_export("solve")
        .context("expect export `solve`")?;
    if ftype.func() != Some(&FuncType::new([ValType::I32], [ValType::I64])) {
        anyhow::bail!("export `solve` does not have signature i32 -> i64");
    }
    let mtype = module
        .get_export("memory")
        .context("expect export `memory`")?;
    if mtype.memory().is_none() {
        anyhow::bail!("export `memory` is not a memory");
    }
    let btype = module
        .get_export("__heap_base")
        .context("expect export `__heap_base`")?;
    if btype.global() != Some(&GlobalType::new(ValType::I32, Mutability::Const)) {
        anyhow::bail!("export `__heap_base` is not a global const i32");
    }

    if module.imports().len() != 0 {
        anyhow::bail!("expected no imports");
    }
}
