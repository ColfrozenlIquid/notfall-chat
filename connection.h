#pragma once

#include <netinet/in.h>
#include <stddef.h>
#include <sys/socket.h>
#include <time.h>
#include <stdint.h>
#include <stdbool.h>

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
} State ;

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

#define SND_BUFFER_SIZE 512
#define RCV_BUFFER_SIZE 512

typedef struct {
    State state;

    uint32_t snd_seq;   // next SEQ number we will send
    uint32_t snd_ack;   // last ACK number we received
    uint32_t rcv_seq;   // next SEQ number we expect to receive

    uint8_t snd_buf[SND_BUFFER_SIZE];
    size_t snd_len;
    uint8_t rcv_buf[RCV_BUFFER_SIZE];
    size_t rcv_len;

    struct timespec time_wait_start;

    int sockfd;

    struct sockaddr_in peer_addr;
    socklen_t peer_len;
} Connection;

typedef void (*ActionFn) (Connection* conn);

typedef struct {
    State current;
    int event;
    State next;
    ActionFn action;
} Transition;

bool fsm_dispatch(Connection* conn, Event event);

void send_syn(Connection* conn);

void send_ack(Connection* conn);

void send_syn_ack(Connection* conn);

void reset_listen(Connection* conn);

void send_fin(Connection* conn);

void send_data(Connection* conn);

void send_segment(Connection* conn, uint16_t flags, uint32_t snd_seq, uint32_t rcv_seq, uint8_t* snd_buf, size_t snd_len);

static const Transition transitions[] = {
    // Client side (receiver)
    { CLOSED, CONNECT, SYN_SENT, send_syn },
    { SYN_SENT, RECV_SYN_ACK, ESTABLISHED, send_ack },

    // Server side (sender)
    { CLOSED, LISTEN_CALL, LISTEN, NULL },
    { LISTEN, RECV_SYN, SYN_RECEIVED, send_syn_ack },
    { SYN_RECEIVED, RECV_ACK, ESTABLISHED, NULL },
    { SYN_RECEIVED, CLOSE, FIN_WAIT_1, send_fin },

    // Data transfer
    { ESTABLISHED, SEND, ESTABLISHED, send_data },

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
