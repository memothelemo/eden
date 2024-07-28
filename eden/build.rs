fn main() {
    println!("cargo::rustc-check-cfg=cfg(release)");
    if let Ok("release") = std::env::var("PROFILE").as_deref() {
        println!("cargo:rustc-cfg=release");
    }
}
