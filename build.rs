fn main() {
    let target = std::env::var("TARGET").unwrap();

    println!("cargo:rerun-if-changed=c_src/devices.c");
    println!("cargo:rerun-if-changed=c_src/discovery.c");
    println!("cargo:rerun-if-changed=c_src/discovery.h");
    println!("cargo:rerun-if-changed=c_src/discovery_listener.c");
    println!("cargo:rerun-if-changed=c_src/discovery_listener.h");
    println!("cargo:rerun-if-changed=c_src/ringbuffer_slotted.h");
    println!("cargo:rerun-if-changed=c_src/ringbuffer_slotted.c");
    println!("cargo:rerun-if-changed=c_src/server.h");
    println!("cargo:rerun-if-changed=c_src/server.c");
    println!("cargo:rerun-if-changed=c_src/packet.h");
    println!("cargo:rerun-if-changed=c_src/packet.c");
    println!("cargo:rerun-if-changed=c_src/connection.h");
    println!("cargo:rerun-if-changed=c_src/connection.c");
    println!("cargo:rerun-if-changed=c_src/ringbuffer.h");
    println!("cargo:rerun-if-changed=c_src/ringbuffer.c");
    println!("cargo:rerun-if-changed=c_src/client.h");
    println!("cargo:rerun-if-changed=c_src/client.c");

    let mut build = cc::Build::new();

    build.files([
        "c_src/devices.c",
        "c_src/discovery.c",
        "c_src/discovery_listener.c",
        "c_src/ringbuffer_slotted.c",
        "c_src/server.c",
        "c_src/packet.c",
        "c_src/connection.c",
        "c_src/ringbuffer.c",
        "c_src/client.c",
    ]);

    if target.contains("musleabihf") {
        build.compiler("arm-linux-musleabihf-gcc");
    } else if target.contains("aarch64-unknown-linux-gnu") {
        build.compiler("aarch64-linux-gnu-gcc");
    }

    build.compile("lib-network");
}
