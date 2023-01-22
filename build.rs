use prql_compiler::{
    compile,
    sql::{Dialect, Options},
};
use std::{env, fs, path::Path};

fn main() {
    // we expect queries to reside in `queries/` dir
    let paths = fs::read_dir("./queries").unwrap();

    // save output to `target/.../out/queries/` dir
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_dir = Path::new(&out_dir).join("queries");

    if !dest_dir.is_dir() {
        fs::create_dir(&dest_dir).unwrap();
    }

    let options = Options::default().with_dialect(Dialect::SQLite).some();
    for path in paths {
        let prql_path = path.unwrap().path();
        let sql_path = dest_dir.join(prql_path.file_name().unwrap());

        let prql_string = fs::read_to_string(prql_path).unwrap();

        let sql_string = compile(&prql_string, options.clone()).unwrap();

        fs::write(sql_path, sql_string).unwrap();
    }
}
