extern crate cbindgen;

use cbindgen::Config;

fn main() {
    let bindings = {
        let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let config = Config::from_file("cbindgen.toml").unwrap();
        cbindgen::generate_with_config(&crate_dir, config).unwrap()
    };
    bindings.write_to_file("include/rbspy.h");
}
