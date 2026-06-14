cmake_minimum_required(VERSION 3.10)
project(notfall-chat C)
set(CMAKE_C_STANDARD 17)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Shared sources (no main)
set(COMMON_SOURCES
    c_src/connection.c
    c_src/packet.c
    c_src/ringbuffer.c
)

add_executable(server
    c_src/main_server.c
    c_src/server.c
    ${COMMON_SOURCES}
)

add_executable(client
    c_src/main_client.c
    c_src/client.c
    c_src/server.c
    ${COMMON_SOURCES}
)
