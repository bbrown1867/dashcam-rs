// If QQVGA resolution not specified, use QVGA resolution
fn main() {
    if !cfg!(feature = "qqvga") {
        println!("cargo:rustc-cfg=feature=\"qvga\"");
    }
}
