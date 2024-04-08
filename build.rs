use rust_query::{client::Client, schema::generate};
use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("tables.rs");

    let client = Client::open_in_memory();
    client.execute_batch(include_str!("src/migration/initial.sql"));
    let code = generate(client);
    fs::write(dest_path, code).unwrap();

    println!("cargo::rerun-if-changed=src/migration/initial.sql");
    println!("cargo::rerun-if-changed=build.rs");
}
