cmake_minimum_required(VERSION 3.10)
project(notfall-chat C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

add_executable(server main.c server.c connection.c packet.c)
add_executable(client main.c client.c connection.c packet.c)
