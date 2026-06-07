#pragma once

#include "packet.h"
#include "connection.h"
#include <stdint.h>

#define PORT 12345
#define BUFFER_SIZE 1024

int run_server(int port);

void handle_packet(Connection* conn, const uint8_t* buf, size_t buf_len);

void* receiver_thread(void* arg);

void* sender_thread(void* arg);
