use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed=DEBRA_EPOCH_CACHE_SIZE");
    println!("cargo:rerun-if-env-changed=DEBRA_CHECK_THRESHOLD");
    println!("cargo:rerun-if-env-changed=DEBRA_ADVANCE_THRESHOLD");

    let out_dir = env::var("OUT_DIR").expect("no out directory");
    let dest = Path::new(&out_dir).join("build_constants.rs");

    let mut file = File::create(&dest).expect("could not create file");

    let check: u32 = option_env!("DEBRA_CHECK_THRESHOLD")
        .map_or(Ok(100), str::parse)
        .expect("failed to parse env variable DEBRA_CHECK_THRESHOLD");

    let advance: u32 = option_env!("DEBRA_ADVANCE_THRESHOLD")
        .map_or(Ok(100), str::parse)
        .expect("failed to parse env variable DEBRA_ADVANCE_THRESHOLD");

    if check == 0 {
        panic!("invalid DEBRA_CHECK_THRESHOLD value (0)");
    }

    write!(&mut file, "const DEBRA_CHECK_THRESHOLD: u32 = {};", check)
        .expect("could not write to file");
    write!(&mut file, "const DEBRA_ADVANCE_THRESHOLD: u32 = {};", advance)
        .expect("could not write to file");
}
