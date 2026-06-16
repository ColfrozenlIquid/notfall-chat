#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include "packet.h"
#include "ringbuffer.h"
#include <stdint.h>
#include <unistd.h>

#include "connection.h"
#include "ringbuffer.h"

Connection* connection_create(void) {
    Connection* conn = calloc(1, sizeof(Connection));
    if (!conn) return NULL;
    pthread_mutex_init(&conn->mutex, NULL);
    pthread_cond_init(&conn->cond_send, NULL);
    pthread_cond_init(&conn->cond_recv, NULL);
    return conn;
}

void connection_destroy(Connection* conn) {
    if (!conn) return;
    pthread_mutex_destroy(&conn->mutex);
    pthread_cond_destroy(&conn->cond_send);
    pthread_cond_destroy(&conn->cond_recv);
    free(conn);
}

int connection_send(Connection* conn, const uint8_t* data, size_t len) {
    if (!conn || !data) return -1;
    write_to_connection(conn, (uint8_t*)data, len);
    return 0;
}

int connection_receive(Connection* conn, uint8_t* dst, size_t* out_len) {
    pthread_mutex_lock(&conn->mutex);
    while (conn->rcv_len == 0) {
        pthread_cond_wait(&conn->cond_recv, &conn->mutex);
    }
    uint32_t data_len = rb_peek_len(&conn->rcv_buf);
    uint8_t buf[8192];
    rb_consume(&conn->rcv_buf, buf);
    memcpy(dst, buf + 4, data_len);
    *out_len = data_len;
    conn->rcv_len = 0;
    pthread_mutex_unlock(&conn->mutex);
    return 0;
}

int connection_try_receive(Connection* conn, uint8_t* dst, size_t* out_len) {
    pthread_mutex_lock(&conn->mutex);
    if (conn->rcv_len == 0) {
        pthread_mutex_unlock(&conn->mutex);
        return 0;
    }

    uint32_t data_len = rb_peek_len(&conn->rcv_buf);
    uint8_t buf[8192];
    rb_consume(&conn->rcv_buf, buf);
    memcpy(dst, buf + 4, data_len);
    *out_len = data_len;
    conn->rcv_len = 0;
    pthread_mutex_unlock(&conn->mutex);
    return 0;
}

void connection_wait(Connection* conn) {
    pthread_join(conn->recv_tid, NULL);
    pthread_join(conn->send_tid, NULL);
}

bool fsm_dispatch(Connection* conn, Event event) {
    for (size_t i = 0; i < sizeof(transitions)/sizeof(transitions[0]); i++) {
        const Transition* t = &transitions[i];
        if (t->current == conn->state && t->event == event) {
            if (t->action) t->action(conn);
            printf("Current state: %s\n", state_str(conn->state));
            conn->state = t->next;
            printf("Changing state to: %s\n", state_str(conn->state));
            return true;
        }
    }
    return false;
}

void send_syn(Connection* conn) {
    conn->snd_seq = rand();
    send_segment(conn, FLAG_SYN, conn->snd_seq, 0, NULL, 0);
    conn->snd_seq++;
}

void send_ack(Connection* conn) {
    send_segment(conn, FLAG_ACK, conn->snd_seq, conn->rcv_seq, NULL, 0);
}

void send_syn_ack(Connection* conn) {
    conn->snd_seq = rand();
    send_segment(conn, FLAG_SYN | FLAG_ACK, conn->snd_seq, conn->rcv_seq, NULL, 0);
    conn->snd_seq++;
}

void send_fin(Connection* conn) {

}

void send_data(Connection* conn) {

}

void notify_established(Connection* conn) {
    if (conn->on_accept) {
        conn->on_accept(conn, conn->on_accept_userdata);
    }
}

void reset_listen(Connection *conn) {}

void send_segment(Connection *conn, uint16_t flags, uint32_t snd_seq, uint32_t rcv_seq, uint8_t* snd_buf, size_t snd_len) {
    Packet pkt;
    memset(&pkt, 0, sizeof(pkt));

    pkt.flags = flags;
    pkt.seq_number = snd_seq;
    pkt.ack_number = rcv_seq;
    pkt.data_len = snd_len;

    if (snd_len > 0) memcpy(pkt.data, snd_buf, snd_len);

    uint8_t buffer[PACKET_HEADER_SIZE + PACKET_DATA_SIZE];
    size_t packet_len = packet_encode(&pkt, buffer, sizeof(buffer));

    sendto(conn->sockfd, buffer, packet_len, 0, (struct sockaddr*)&conn->peer_addr, conn->peer_len);
}

const char* state_str(State s) {
    if (s < 0 || s >= STATE_COUNT)
        return "UNKNOWN_STATE";
    return state_to_string[s];
}

void write_to_connection(Connection* conn, uint8_t* data, size_t data_len) {
    uint8_t header[4];
    header[0] = (data_len >> 24) & 0xFF;
    header[1] = (data_len >> 16) & 0xFF;
    header[2] = (data_len >> 8) & 0xFF;
    header[3] = data_len & 0xFF;

    size_t total_len = 4 + data_len;

    pthread_mutex_lock(&conn->mutex);
    while (conn->snd_len + total_len > SND_BUFFER_SIZE) {
        pthread_cond_wait(&conn->cond_send, &conn->mutex);
    }

    // memcpy(conn->snd_buf + conn->snd_len, header, 4);
    // memcpy(conn->snd_buf + conn->snd_len + 4, data, data_len);
    rb_write(&conn->snd_buf, header, 4);
    rb_write(&conn->snd_buf, data, data_len);
    conn->snd_len += total_len;

    pthread_mutex_unlock(&conn->mutex);
    pthread_cond_signal(&conn->cond_send);
}
