#include <arpa/inet.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include "client.h"
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

    pthread_join(recv_tid, NULL);
    pthread_join(send_tid, NULL);
    return 0;
}
