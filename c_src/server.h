#pragma once
#include "packet.h"
#include "connection.h"
#include <stdint.h>

#define PORT 12345
#define BUFFER_SIZE 1024
#define MAX_RETRIES 5
#define RTO_MS 200

typedef void (*on_accept_cb)(Connection* conn, void* userdata);

int run_server(int port, on_accept_cb cb, void* userdata);

void handle_incoming_packet(Connection* conn, const uint8_t* buf, size_t buf_len);

void* receiver_thread(void* arg);

void* sender_thread(void* arg);

static int should_drop(float loss_rate);
