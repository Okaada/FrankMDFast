//! Windows only: embed the app icon and version info in `frankmdfast.exe`.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=../../assets/brand/icon.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/brand/icon.ico")
            .set("ProductName", "FrankMDFast")
            .set("FileDescription", "FrankMDFast markdown editor")
            .set("InternalName", "frankmdfast");
        if let Err(err) = res.compile() {
            panic!("failed to embed Windows resources: {err}");
        }
    }
}
