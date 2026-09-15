fn main() {
    // asset-importer-sys emits a dynamic stdc++ link on MinGW. Bundle it so
    // Explorer can load the DLL on machines without a C++ development toolchain.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu")
    {
        let compiler = cc::Build::new().cpp(true).get_compiler();
        let output = compiler
            .to_command()
            .arg("-print-file-name=libstdc++.a")
            .output()
            .expect("cannot locate MinGW static C++ runtime");
        assert!(output.status.success(), "C++ compiler query failed");
        let archive = String::from_utf8(output.stdout).expect("invalid compiler output");
        let archive = std::path::Path::new(archive.trim());
        assert!(archive.is_file(), "MinGW libstdc++.a is missing");
        println!(
            "cargo:rustc-link-search=native={}",
            archive.parent().unwrap().display()
        );
        println!("cargo:rustc-link-lib=static:+whole-archive=stdc++");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
