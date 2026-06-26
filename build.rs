fn main() {
    // Only link libguile when cross-compiling the x86_64 cdylib for Guile.
    // Native arm64 builds (cargo run) don't need it.
    let target = std::env::var("TARGET").unwrap_or_default();
    if target == "x86_64-apple-darwin" {
        println!("cargo:rustc-link-search=native=/usr/local/lib");
        println!("cargo:rustc-link-lib=dylib=guile-3.0");
        println!("cargo:rustc-link-search=native=/usr/local/opt/bdw-gc/lib");
        println!("cargo:rustc-link-lib=dylib=gc");
    }
}
