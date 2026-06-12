build: clean
	mkdir -p build && cd build && cmake .. && make

run: build
	./build/server

run-client:
	./build/client 127.0.0.1 12345

clean:
	rm -rf build
