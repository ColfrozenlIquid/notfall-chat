fn main() {
    println!("cargo:rerun-if-changed=c_src/devices.c");
    println!("cargo:rerun-if-changed=c_src/discovery.c");
    println!("cargo:rerun-if-changed=c_src/discovery.h");
    println!("cargo:rerun-if-changed=c_src/discovery_listener.c");
    println!("cargo:rerun-if-changed=c_src/discovery_listener.h");
    println!("cargo:rerun-if-changed=c_src/ringbuffer_slotted.h");
    println!("cargo:rerun-if-changed=c_src/ringbuffer_slotted.c");

    cc::Build::new()
        .files([
            "c_src/devices.c",
            "c_src/discovery.c",
            "c_src/discovery_listener.c",
            "c_src/ringbuffer_slotted.c",
        ])
        .compile("lib-network");
}
