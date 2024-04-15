use advent_of_wasm::solution::Solution;
use wasmtime::{Config, Engine};

fn main() {
    let sol = Solution {
        hash: "jm2mh0Qgr74".parse().unwrap(),
    };

    let engine = Engine::new(Config::new().consume_fuel(true)).unwrap();
    let data = "1234".as_bytes();
    let res = sol.run(&engine, data, 10000);
    println!("{:?}", res.answer);
}
