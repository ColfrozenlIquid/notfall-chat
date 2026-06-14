fn main() {
    let target = std::env::var("TARGET").unwrap();

    println!("cargo:rerun-if-changed=c_src/devices.c");
    println!("cargo:rerun-if-changed=c_src/discovery.c");
    println!("cargo:rerun-if-changed=c_src/discovery.h");
    println!("cargo:rerun-if-changed=c_src/discovery_listener.c");
    println!("cargo:rerun-if-changed=c_src/discovery_listener.h");
    println!("cargo:rerun-if-changed=c_src/ringbuffer_slotted.h");
    println!("cargo:rerun-if-changed=c_src/ringbuffer_slotted.c");

    let mut build = cc::Build::new();

    build.files([
        "c_src/devices.c",
        "c_src/discovery.c",
        "c_src/discovery_listener.c",
        "c_src/ringbuffer_slotted.c",
    ]);

    if target.contains("musleabihf") {
        build.compiler("arm-linux-musleabihf-gcc");
    }

    build.compile("lib-network");
}
