#pragma once
#include "connection.h"

int connect_to_server(const char* server_ip, int port, Connection* conn);

uint8_t* read_input(int* out_len);
