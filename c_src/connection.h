#pragma once
#include <bits/pthreadtypes.h>
#include <bits/types/struct_iovec.h>
#include <netinet/in.h>
#include <stddef.h>
#include <sys/socket.h>
#include <time.h>
#include <stdint.h>
#include <stdbool.h>

#include "ringbuffer.h"

typedef enum {
    CLOSED,
    LISTEN,
    SYN_RECEIVED,
    SYN_SENT,
    ESTABLISHED,
    FIN_WAIT_1,
    FIN_WAIT_2,
    CLOSING,
    TIME_WAIT,
    CLOSE_WAIT,
    LAST_ACK,
    STATE_COUNT,
} State ;

static const char* state_to_string[] = {
    "CLOSED",
    "LISTEN",
    "SYN_RECEIVED",
    "SYN_SENT",
    "ESTABLISHED",
    "FIN_WAIT_1",
    "FIN_WAIT_2",
    "CLOSING",
    "TIME_WAIT",
    "CLOSE_WAIT",
    "LAST_ACK"
};

const char* state_str(State s);

typedef enum {
    CONNECT,
    RECV_SYN,
    RECV_SYN_ACK,
    RECV_FIN_ACK,
    RECV_ACK,
    RECV_RST,
    CLOSE,
    LISTEN_CALL,
    SEND,
    RECV_FIN,
    TIMEOUT,
} Event;

#define SND_BUFFER_SIZE 8196
#define RCV_BUFFER_SIZE 8196

typedef struct Connection Connection;

typedef void (*on_accept_cb)(Connection* conn, void* userdata);

typedef struct Connection {
    State state;

    uint32_t snd_seq;   // next SEQ number we will send
    uint32_t snd_ack;   // last ACK number we received
    uint32_t rcv_seq;   // next SEQ number we expect to receive

    RingBuffer snd_buf;
    RingBuffer rcv_buf;

    size_t peer_window;

    struct timespec time_wait_start;

    int sockfd;

    struct sockaddr_in peer_addr;
    socklen_t peer_len;

    pthread_mutex_t mutex;
    pthread_cond_t cond_send;
    pthread_cond_t cond_recv;

    int receiver_ready;

    pthread_t recv_tid;
    pthread_t send_tid;

    on_accept_cb on_accept;
    void* on_accept_userdata;
} Connection;

typedef void (*ActionFn) (Connection* conn);

typedef struct {
    State current;
    int event;
    State next;
    ActionFn action;
} Transition;

size_t connection_rcv_window_size(Connection* conn);

Connection* connection_create(void);

void connection_destroy(Connection* conn);

int connection_send(Connection* conn, const uint8_t* data, size_t len);

int connection_receive(Connection* conn, uint8_t* dst, size_t* out_len);

int connection_try_receive(Connection* conn, uint8_t* dst, size_t* out_len);

void connection_wait(Connection* conn);

bool fsm_dispatch(Connection* conn, Event event);

void send_syn(Connection* conn);

void send_ack(Connection* conn);

void send_syn_ack(Connection* conn);

void reset_listen(Connection* conn);

void send_fin(Connection* conn);

void send_data(Connection* conn);

void send_segment(Connection* conn, uint16_t flags, uint32_t snd_seq, uint32_t rcv_seq, uint8_t* snd_buf, size_t snd_len);

void write_to_connection(Connection* conn, uint8_t* data, size_t data_len);

void notify_established(Connection* conn);

static const Transition transitions[] = {
    // Client side (receiver)
    { CLOSED, CONNECT, SYN_SENT, send_syn },
    { SYN_SENT, RECV_SYN_ACK, ESTABLISHED, send_ack },

    // Server side (sender)
    { CLOSED, LISTEN_CALL, LISTEN, NULL },
    { LISTEN, RECV_SYN, SYN_RECEIVED, send_syn_ack },
    { SYN_RECEIVED, RECV_ACK, ESTABLISHED, notify_established },
    { SYN_RECEIVED, CLOSE, FIN_WAIT_1, send_fin },

    // Data transfer
    { ESTABLISHED, SEND, ESTABLISHED, NULL },

    // Active close
    { ESTABLISHED, CLOSE, FIN_WAIT_1, send_fin },
    { FIN_WAIT_1, RECV_ACK, FIN_WAIT_2, NULL },
    { FIN_WAIT_2, RECV_FIN, TIME_WAIT, send_ack },
    { TIME_WAIT, TIMEOUT, CLOSED, NULL },

    // Passive close
    { CLOSE_WAIT, CLOSE, LAST_ACK, send_fin },
    { ESTABLISHED, RECV_FIN, CLOSE_WAIT, send_ack },
    { LAST_ACK, RECV_ACK, CLOSED, NULL },

    // Unusual Path
    { SYN_RECEIVED, RECV_RST, LISTEN, reset_listen },
    { LISTEN, CLOSE, CLOSED, NULL },
    { LISTEN, SEND, SYN_SENT, send_syn },
    { SYN_SENT, CLOSE, CLOSED, NULL },
    { SYN_SENT, RECV_SYN, SYN_RECEIVED, send_syn_ack },
    { SYN_RECEIVED, CLOSE, FIN_WAIT_1, send_fin },
    { FIN_WAIT_1, RECV_FIN, CLOSING, send_ack },
    { FIN_WAIT_1, RECV_FIN_ACK, TIME_WAIT, send_ack },
    { CLOSING, RECV_ACK, TIME_WAIT, NULL },
};
