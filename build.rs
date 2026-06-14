fn main() {
    println!("cargo:rerun-if-changed=c_src/devices.c");
    println!("cargo:rerun-if-changed=c_src/discovery.c");
    println!("cargo:rerun-if-changed=c_src/discovery.h");

    cc::Build::new()
        .files(["c_src/devices.c", "c_src/discovery.c"])
        .compile("lib-network");
}
