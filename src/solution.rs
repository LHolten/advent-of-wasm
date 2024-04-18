use fehler::{throw, throws};
use wasmtime::{
    Engine, FuncType, GlobalType, Linker, Module, Mutability, Store, TypedFunc, ValType,
};

use crate::{hash::FileHash, problem::ModulePath};

pub struct Solution {
    pub hash: FileHash,
}

#[derive(Debug)]
pub struct Run {
    pub fuel_used: u64,
    pub answer: i64,
}

impl Solution {
    pub fn run(&self, engine: &Engine, data: &[u8], fuel: u64) -> Result<Run, String> {
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

        drop(memory.grow(&mut store, 1));
        drop(memory.write(&mut store, heap_base as usize, data));

        // call the actual solve function
        let answer = func
            .call(&mut store, data.len() as i32)
            .map_err(|e| e.to_string())?;

        Ok(Run {
            fuel_used: store.fuel_consumed().unwrap(),
            answer,
        })
    }
}

#[throws(String)]
pub fn verify_wasm(buf: &[u8]) {
    let engine = Engine::default();
    let module = Module::from_binary(&engine, buf).map_err(|e| e.to_string())?;

    let ftype = module
        .get_export("solve")
        .ok_or("there is no export `solve`")?;
    if ftype.func() != Some(&FuncType::new([ValType::I32], [ValType::I64])) {
        throw!("export `solve` does not have signature i32 -> i64");
    }
    let mtype = module
        .get_export("memory")
        .ok_or("there is no export `memory`")?;
    if mtype.memory().is_none() {
        throw!("export `memory` is not a memory");
    }
    let btype = module
        .get_export("__heap_base")
        .ok_or("there is no export `__heap_base`")?;
    if btype.global() != Some(&GlobalType::new(ValType::I32, Mutability::Const)) {
        throw!("export `__heap_base` is not a global const i32");
    }

    if module.imports().len() != 0 {
        throw!("expected no imports");
    }
}
