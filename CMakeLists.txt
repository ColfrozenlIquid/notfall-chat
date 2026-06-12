cmake_minimum_required(VERSION 3.10)
project(notfall-chat C)
set(CMAKE_C_STANDARD 11)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Shared sources (no main)
set(COMMON_SOURCES
    connection.c
    packet.c
)

add_executable(server
    main_server.c
    server.c
    ringbuffer.c
    ${COMMON_SOURCES}
)

add_executable(client
    main_client.c
    client.c
    server.c
    ringbuffer.c
    ${COMMON_SOURCES}
)
