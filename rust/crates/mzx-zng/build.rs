fn main() {
    let dst = cmake::Config::new("zlib-ng")
        .define("ZLIB_COMPAT", "OFF")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("ZLIB_ENABLE_TESTS", "OFF")
        .define("ZLIBNG_ENABLE_TESTS", "OFF")
        .define("WITH_GTEST", "OFF")
        .define("WITH_FUZZERS", "OFF")
        .define("WITH_BENCHMARKS", "OFF")
        .define("CMAKE_BUILD_TYPE", "Release")
        .build();
    cc::Build::new()
        .file("shim/dotnet_zlib.c")
        .include(dst.join("include"))
        .compile("mzx_zng_shim");
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib").display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("lib64").display()
    );
    println!("cargo:rustc-link-lib=static=z-ng");
    println!("cargo:rerun-if-changed=shim/dotnet_zlib.c");
    println!("cargo:rerun-if-changed=zlib-ng/VENDORED");
}
