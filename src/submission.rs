use wasmtime::{Engine, Linker, Module, Store, TypedFunc};

pub struct Submission {
    author: String,
    hash: String,
}

impl Submission {
    pub fn run_submission(&self, engine: &Engine, data: &[u8], fuel: u64) -> Box<[u8]> {
        // let module = Module::new(engine, bytes).unwrap();
        // let mut store = Store::new(engine, ());

        // // first instantiate, this calls optional start
        // store.add_fuel(fuel).unwrap();
        // let instance = Linker::new(engine)
        //     .instantiate(&mut store, &module)
        //     .unwrap();

        // // now we can write the actual input
        // let memory = instance.get_memory(&mut store, "memory").unwrap();
        // memory.write(&mut store, 0, data).unwrap();
        // let input = data.len() as i32;

        // // add some fuel here so the program can run
        // let func: TypedFunc<_, (i32, i32)> = instance.get_typed_func(&mut store, "solve").unwrap();
        // let (offset, length) = func.call(&mut store, input).unwrap();

        // let mut output = vec![0; length as usize].into_boxed_slice();
        // memory.read(&store, offset as usize, &mut *output).unwrap();
        // output
        todo!()
    }
}
