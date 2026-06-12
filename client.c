#include <arpa/inet.h>
#include <bits/types/struct_iovec.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>
#include <stdlib.h>

#include "client.h"
#include "connection.h"
#include "server.h"

int connect_to_server(const char* server_ip, int port, Connection* conn) {
    int sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd < 0) {
        perror("socket error");
        return -1;
    }

    struct sockaddr_in local_addr = {0};
    local_addr.sin_family      = AF_INET;
    local_addr.sin_addr.s_addr = INADDR_ANY;
    local_addr.sin_port        = 0;
    if (bind(sockfd, (struct sockaddr*)&local_addr, sizeof(local_addr)) < 0) {
        perror("client bind");
        close(sockfd);
        return -1;
    }

    struct sockaddr_in server_addr = {0};
        server_addr.sin_family = AF_INET;
        server_addr.sin_port   = htons(port);
        if (inet_pton(AF_INET, server_ip, &server_addr.sin_addr) <= 0) {
            perror("inet_pton");
            close(sockfd);
            return -1;
        }

    memset(conn, 0, sizeof(*conn));
    conn->sockfd    = sockfd;
    conn->peer_addr = server_addr;
    conn->peer_len  = sizeof(server_addr);
    conn->state     = CLOSED;
    pthread_mutex_init(&conn->mutex,    NULL);
    pthread_cond_init(&conn->cond_send, NULL);
    pthread_cond_init(&conn->cond_recv, NULL);

    pthread_t recv_tid, send_tid;
    pthread_create(&recv_tid, NULL, receiver_thread, conn);
    pthread_create(&send_tid, NULL, sender_thread,   conn);

    pthread_mutex_lock(&conn->mutex);
    while (!conn->receiver_ready) {
        pthread_cond_wait(&conn->cond_recv, &conn->mutex);
    }

    fsm_dispatch(conn, CONNECT);

    while (conn->state != ESTABLISHED) {
        pthread_cond_wait(&conn->cond_recv, &conn->mutex);
    }
    pthread_mutex_unlock(&conn->mutex);

    printf("connection established\n");

    while(1) {
        int len = 0;
        uint8_t* input = read_input(&len);
        if(!input) break;
        write_to_connection(conn, input, len);
        free(input);
    }

    pthread_join(recv_tid, NULL);
    pthread_join(send_tid, NULL);
    return 0;
}

uint8_t* read_input(int* out_len) {
    size_t capacity = 64;
    size_t length = 0;
    uint8_t* buf = (uint8_t*)malloc(capacity);
    if (!buf) {
        if (out_len) *out_len = 0;
        return NULL;
    }

    int c;
    while((c = getchar()) != '\n' && c != EOF) {
        if (length + 1 >= capacity) {
            capacity *= 2;
            uint8_t* tmp = (uint8_t*)realloc(buf, capacity);
            if (!tmp) {
                free(buf);
                if (out_len) *out_len = 0;
                return NULL;
            }
            buf = tmp;
        }
        buf[length++] = (char)c;
    }
    buf[length] = '\0';
    if (out_len) *out_len = (int)length;
    return buf;
}
