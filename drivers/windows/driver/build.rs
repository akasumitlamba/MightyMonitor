fn main() {
    winresource::WindowsResource::new()
        .set("ProductName", "MightyMonitor Display")
        .set(
            "FileDescription",
            "Experimental MightyMonitor indirect display driver",
        )
        .compile()
        .expect("compile driver version resource");
    println!("cargo:rustc-link-lib=static=ucrt");
    println!("cargo:rerun-if-changed=build.rs");
}
