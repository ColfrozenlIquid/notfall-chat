#pragma once

#include "packet.h"
#include "connection.h"

#define PORT 12345
#define BUFFER_SIZE 1024

int run_server();

void handle_packet(Connection* conn, const char* buf, size_t buf_len, int sockfd);
