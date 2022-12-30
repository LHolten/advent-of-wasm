use fehler::throws;
use wasmtime::*;

// mod data;
// mod db;
// mod id;
mod migration;
mod submission;
mod task;

struct Server {
    engine: Engine,
    task_dir: task::TaskDir,
}

impl Server {
    #[throws(anyhow::Error)]
    pub fn new(task: task::Task) -> Self {
        let mut config = Config::new();
        config
            .consume_fuel(true)
            .cranelift_opt_level(OptLevel::None);
        let engine = Engine::new(&config).unwrap();

        Self {
            engine,
            task_dir: task::TaskDir::new()?,
        }
    }
}

#[cfg(tests)]
#[test]
fn try_identity() {
    let data = vec![1, 2, 3, 42].into_boxed_slice();
    let task = task::Task {
        input: data.clone(),
        output: data,
        fuel: 100,
    };
    let server = Server::new(task);

    let wat = r#"
    (module
        (type $t0 (func (param i32) (result i32 i32)))
        (type $t1 (func (param) (result)))
        (func $solve (export "solve") (type $t0) (param $p0 i32) (result i32 i32)
          (i32.const 0)
          (local.get $p0))
        (func $hello (export "hello") (type $t1))
        (memory $memory (export "memory") 16)
        (global $__stack_pointer (mut i32) (i32.const 1048576))
        (global $__data_end (export "__data_end") i32 (i32.const 1048576))
        (global $__heap_base (export "__heap_base") i32 (i32.const 1048576))
        (start $hello)
    )
    "#;
    server.run_submission(wat)
}
