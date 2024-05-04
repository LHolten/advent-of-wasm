use std::{ffi::CStr, str::from_utf8};

use anyhow::{anyhow, bail};
use fehler::{throw, throws};

use wasmtime::{Caller, Config, Engine, Instance, Linker, Module, Store, Trap, Val};

use crate::{
    hash::FileHash,
    problem::{ModulePath, Problem},
};

pub struct Solution {
    pub hash: FileHash,
}

impl Solution {
    pub fn run(&self, problem: &Problem, seed: i64) -> Result<u64, String> {
        let solution_engine = Engine::new(Config::new().consume_fuel(true)).unwrap();

        let path = format!("solution/{}.wasm", &self.hash);
        let solution = ModulePath(path.into()).load(&solution_engine).unwrap();

        // first instantiate, this calls optional start
        // add some fuel here so the program can run
        let mut store = Store::new(&solution_engine, ());
        store.set_fuel(problem.fuel_limit).unwrap();
        let solution = Linker::new(&solution_engine)
            .instantiate(&mut store, &solution)
            .map_err(|e| format!("{e:?}"))?;

        let data = Data {
            store,
            solution,
            stack: vec![],
            error: None,
            used_heap_base: false,
        };

        let problem_engine = Engine::default();
        let problem_module = problem.file_name.load(&problem_engine).unwrap();

        let mut problem_store = Store::new(&problem_engine, data);
        let problem_instance = add_env(&mut Linker::new(&problem_engine))
            .instantiate(&mut problem_store, &problem_module)
            .unwrap();

        let func = problem_instance
            .get_typed_func::<i64, ()>(&mut problem_store, "test")
            .unwrap();

        let call = func.call(&mut problem_store, seed);
        if let Err(e) = call {
            if let Some(e) = &problem_store.data().error {
                if problem_store.data().used_heap_base {
                    throw!(format!("NOTE: `__heap_base` was used because `string_base` is undefined \n solution error: \n {e:?}"))
                }
                throw!(format!("solution error: \n {e:?}"))
            } else if e.is::<Trap>() {
                throw!(format!("problem error: \n {e:?}"))
            } else {
                throw!(e.root_cause().to_string())
            }
        } else {
            Ok(problem.fuel_limit - problem_store.data().store.get_fuel().unwrap())
        }
    }
}

struct Data {
    store: Store<()>,
    solution: Instance,
    stack: Vec<Val>,
    used_heap_base: bool,
    error: Option<anyhow::Error>,
}

fn add_env(linker: &mut Linker<Data>) -> &mut Linker<Data> {
    linker
        .func_wrap("env", "heap_base", move |mut caller: Caller<'_, Data>| {
            let Data {
                store,
                solution,
                used_heap_base,
                ..
            } = caller.data_mut();
            let string_base = solution.get_global(&mut *store, "string_base");
            if let Some(string_base) = string_base {
                return string_base
                    .get(&mut *store)
                    .i32()
                    .ok_or(anyhow!("export `string_base` does not have type i32"));
            }
            // make sure that old solutions still work
            let heap_base = solution
                .get_global(&mut *store, "__heap_base")
                .and_then(|x| x.get(&mut *store).i32());
            if let Some(heap_base) = heap_base {
                *used_heap_base = true;
                return Ok(heap_base);
            }
            bail!("no global called `string_base` exists");
        })
        .unwrap()
        .func_wrap(
            "env",
            "load8",
            move |mut caller: Caller<'_, Data>, ptr: i32| {
                let Data {
                    store, solution, ..
                } = caller.data_mut();
                let mem = solution
                    .get_memory(&mut *store, "memory")
                    .ok_or(anyhow!("export `memory` is not a memory"))?;

                let mut buf = vec![0];
                mem.read(&mut *store, ptr as usize, &mut buf)
                    .map_err(|_| anyhow!("can not read outside the memory"))?;
                Ok(buf[0] as i32)
            },
        )
        .unwrap()
        .func_wrap(
            "env",
            "store8",
            move |mut caller: Caller<'_, Data>, ptr: i32, val: i32| {
                let Data {
                    store, solution, ..
                } = caller.data_mut();
                let mem = solution
                    .get_memory(&mut *store, "memory")
                    .ok_or(anyhow!("export `memory` is not a memory"))?;
                let size = mem.size(&mut *store);
                let pages_needed = ptr as u64 / 65536 + 1;
                if pages_needed > size {
                    mem.grow(&mut *store, pages_needed - size)
                        .map_err(|_| anyhow!("could not grow memory"))?;
                }

                mem.write(&mut *store, ptr as usize, &[val as u8]).unwrap();
                Ok(())
            },
        )
        .unwrap()
        .func_wrap(
            "env",
            "bench",
            |mut caller: Caller<'_, Data>, ptr: i32, rets: i32| {
                let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
                let name = CStr::from_bytes_until_nul(&mem.data(&caller)[ptr as usize..]).unwrap();
                let name = name.to_str().unwrap().to_owned();
                let Data {
                    store,
                    solution,
                    stack,
                    error,
                    ..
                } = caller.data_mut();
                let func = solution
                    .get_func(&mut *store, &name)
                    .ok_or(anyhow!("no function export called {name}"))?;
                let mut ret = vec![Val::null_func_ref(); rets as usize];
                if let Err(e) = func.call(store, stack, &mut ret) {
                    *error = Some(e);
                    anyhow::bail!("error is stored elsewhere")
                }
                *stack = ret;
                Ok(())
            },
        )
        .unwrap()
        .func_wrap("env", "push64", |mut caller: Caller<'_, Data>, val: i64| {
            caller.data_mut().stack.push(Val::I64(val))
        })
        .unwrap()
        .func_wrap("env", "push32", |mut caller: Caller<'_, Data>, val: i32| {
            caller.data_mut().stack.push(Val::I32(val))
        })
        .unwrap()
        .func_wrap("env", "pop64", |mut caller: Caller<'_, Data>| {
            let val = caller.data_mut().stack.pop().unwrap();
            val.i64().ok_or(anyhow!("expected function to return i64"))
        })
        .unwrap()
        .func_wrap("env", "pop32", |mut caller: Caller<'_, Data>| {
            let val = caller.data_mut().stack.pop().unwrap();
            val.i32().ok_or(anyhow!("expected function to return i32"))
        })
        .unwrap()
        .func_wrap(
            "env",
            "throw",
            |mut caller: Caller<'_, Data>, ptr: i32, len: i32| -> anyhow::Result<()> {
                let mem = caller.get_export("memory").unwrap().into_memory().unwrap();
                let bytes = &mem.data(&caller)[ptr as usize..][..len as usize];
                let msg = from_utf8(bytes).unwrap();
                anyhow::bail!("{msg}");
            },
        )
        .unwrap()
}

#[throws(String)]
pub fn verify_wasm(buf: &[u8]) {
    let engine = Engine::default();

    Module::validate(&engine, buf).map_err(|e| e.to_string())?;
}

#[cfg(test)]
mod tests {

    use wasmtime::Config;

    use super::*;

    fn get_data() -> Data {
        let engine = Engine::new(Config::new().consume_fuel(true)).unwrap();

        let solution = Module::from_file(&engine, "fast.wasm").unwrap();
        let linker = Linker::new(&engine);
        let mut store = Store::new(&engine, ());
        store.set_fuel(3000).unwrap();

        Data {
            solution: linker.instantiate(&mut store, &solution).unwrap(),
            store,
            stack: vec![],
            error: None,
            used_heap_base: false,
        }
    }

    #[test]
    fn test_stuff() {
        let data = get_data();

        let engine = Engine::new(&Config::new()).unwrap();

        let problem = Module::from_file(&engine, "problem/decimal_new.wasm").unwrap();

        let mut store = Store::new(&engine, data);
        let instance = add_env(&mut Linker::new(&engine))
            .instantiate(&mut store, &problem)
            .unwrap();
        let test = instance
            .get_typed_func::<u64, u64>(&mut store, "test")
            .unwrap();
        let err = test.call(&mut store, u64::MAX).unwrap();
        assert_eq!(err, 0);

        let Data { store, .. } = store.data_mut();
        let fuel = 3000 - store.get_fuel().unwrap();
        println!("{fuel}")
    }
}
