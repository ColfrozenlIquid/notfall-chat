#include <bits/time.h>
#include <bits/types/struct_iovec.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <errno.h>

#include "server.h"
#include "connection.h"
#include "ringbuffer.h"

int run_server(int port, on_accept_cb cb, void* userdata){
    int sockfd;
    struct sockaddr_in server_addr;

    sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd < 0) {
        perror("socket error");
        return 1;
    }

    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = INADDR_ANY;
    server_addr.sin_port = htons(port);

    if (bind(sockfd, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        perror("bind socket");
        close(sockfd);
        return 1;
    }

    printf("UDP server socket listening on port: %d\n", port);

    // Connection conn = {0};
    Connection* conn = calloc(1, sizeof(Connection));
    conn->on_accept = cb;
    conn->on_accept_userdata = userdata;
    conn->sockfd = sockfd;
    conn->state = LISTEN;
    rb_init(&conn->snd_buf);
    rb_init(&conn->rcv_buf);

    pthread_mutex_init(&conn->mutex, NULL);
    pthread_cond_init(&conn->cond_recv, NULL);
    pthread_cond_init(&conn->cond_send, NULL);

    pthread_t recv_thread;
    pthread_t send_thread;
    pthread_create(&recv_thread, NULL, receiver_thread, conn);
    pthread_create(&send_thread, NULL, sender_thread, conn);

    pthread_join(recv_thread, NULL);
    pthread_join(send_thread, NULL);

    pthread_mutex_destroy(&conn->mutex);
    pthread_cond_destroy(&conn->cond_send);
    pthread_cond_destroy(&conn->cond_recv);
    close(sockfd);
    return 0;
}

void handle_incoming_packet(Connection* conn, const uint8_t* buf, size_t buf_len) {
    printf("Handling incoming packet\n");
    Packet packet;
    if (packet_decode(buf, buf_len, &packet) != 0) {
        fprintf(stderr, "invalid packet (bad length or checksum)\n");
        return;
    }

    printf("Packet data len: %d\n", packet.data_len);
    for (int i = 0; i < packet.data_len; i++) {
        printf("[%hhu]", packet.data[i]);
    }
    printf("\n");

    conn->snd_ack = packet.ack_number;
    if (packet.flags & FLAG_SYN || packet.flags & FLAG_FIN) {
        conn->rcv_seq = packet.seq_number + 1;
    } else if (packet.data_len > 0) {
        conn->rcv_seq = packet.seq_number + packet.data_len;
        rb_write(&conn->rcv_buf, packet.data, packet.data_len);
        conn->rcv_len = packet.data_len;
        pthread_cond_signal(&conn->cond_recv);
        send_ack(conn);
    }

    Event event;
    if (packet.flags & FLAG_RST) event = RECV_RST;
    else if ((packet.flags & FLAG_SYN) && (packet.flags & FLAG_ACK)) event = RECV_SYN_ACK;
    else if (packet.flags & FLAG_SYN) event = RECV_SYN;
    else if ((packet.flags & FLAG_FIN) && (packet.flags & FLAG_ACK)) event = RECV_FIN_ACK;
    else if (packet.flags & FLAG_FIN) event = RECV_FIN;
    else if (packet.flags & FLAG_ACK) event = RECV_ACK;
    else {
        fprintf(stderr, "unknown flags: 0x%04x\n", packet.flags);
        return;
    }

    printf("Dispatching\n");

    fsm_dispatch(conn, event);
}

void* receiver_thread(void* arg) {
    Connection* conn = (Connection*)arg;
    uint8_t buffer[BUFFER_SIZE];
    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);

    pthread_mutex_lock(&conn->mutex);
    conn->receiver_ready = 1;
    pthread_cond_signal(&conn->cond_recv);
    pthread_mutex_unlock(&conn->mutex);

    while (1) {
        ssize_t bytes = recvfrom(
            conn->sockfd,
            buffer,
            sizeof(buffer) - 1,
            0,
            (struct sockaddr*)&client_addr,
            &client_len
        );

        if (bytes < 0) {
            perror("recvfrom");
            continue;
        }

        pthread_mutex_lock(&conn->mutex);
        conn->peer_addr = client_addr;
        conn->peer_len = client_len;

        handle_incoming_packet(conn, buffer, bytes);

        pthread_cond_signal(&conn->cond_send);
        pthread_cond_signal(&conn->cond_recv);
        pthread_mutex_unlock(&conn->mutex);
    }
}

void* sender_thread(void* arg) {
    Connection* conn = (Connection*)arg;

    pthread_mutex_lock(&conn->mutex);

    while (1) {
        while (conn->snd_len == 0) {
            pthread_cond_wait(&conn->cond_send, &conn->mutex);
        }

        uint8_t buf[BUFFER_SIZE];
        size_t len = conn->snd_len;
        uint32_t seq = conn->snd_seq;
        uint32_t ack = conn->rcv_seq;
        // memcpy(buf, conn->snd_buf, len);
        rb_consume(&conn->snd_buf, buf);

        pthread_mutex_unlock(&conn->mutex);
        send_segment(conn, FLAG_ACK, seq, ack, buf, len);
        pthread_mutex_lock(&conn->mutex);

        uint32_t expected_ack = seq + len;

        struct timespec deadline;
        int retries = 0;

        while (conn->snd_ack != expected_ack && retries < MAX_RETRIES) {
            clock_gettime(CLOCK_REALTIME, &deadline);
            deadline.tv_nsec += RTO_MS * 1000000L;
            if (deadline.tv_nsec >= 1000000000L) {
                deadline.tv_sec++;
                deadline.tv_nsec -= 1000000000L;
            }

            int rc = pthread_cond_timedwait(&conn->cond_send, &conn->mutex, &deadline);
            if (rc == ETIMEDOUT && conn->snd_ack != expected_ack) {
                pthread_mutex_unlock(&conn->mutex);
                send_segment(conn, FLAG_ACK, seq, ack, buf, len);
                pthread_mutex_lock(&conn->mutex);
                retries++;
            }
        }

        if (conn->snd_ack == expected_ack) {
            conn->snd_seq += len;
            conn->snd_len = 0;
        } else {
            fprintf(stderr, "send failed after %d retries\n", MAX_RETRIES);
        }
    }
    pthread_mutex_unlock(&conn->mutex);
    return NULL;
}
