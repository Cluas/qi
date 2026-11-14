fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/ios/qi_healthkit_bridge.m");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        cc::Build::new()
            .file("src/ios/qi_healthkit_bridge.m")
            .flag("-fobjc-arc")
            .compile("qi_healthkit_bridge");

        println!("cargo:rustc-link-lib=framework=HealthKit");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}
