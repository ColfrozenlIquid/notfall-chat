build: clean
	mkdir -p build && cd build && cmake .. && make

run: build
	./build/server

run-client:
	./build/client 127.0.0.1 12345

clean:
	rm -rf build

build-raspberry:
	cargo build --release --target aarch64-unknown-linux-gnu
	scp target/aarch64-unknown-linux-gnu/release/notfall-chat pi@192.168.1.135:~/
