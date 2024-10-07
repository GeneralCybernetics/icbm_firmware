fn main() {
    let target = std::env::var("TARGET").unwrap();
    if target.starts_with("thumb") {
        println!("cargo:rustc-link-arg-bins=--nmagic");
        println!("cargo:rustc-link-arg-bins=-Tlink.x");
        println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    }
}
