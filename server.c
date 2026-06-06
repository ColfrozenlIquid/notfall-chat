#include <bits/types/struct_iovec.h>
#include <netinet/in.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>
#include <arpa/inet.h>

#include "server.h"
#include "connection.h"

int run_server(int port) {
    int sockfd;
    uint8_t buffer[BUFFER_SIZE];

    struct sockaddr_in server_addr;
    struct sockaddr_in client_addr;

    socklen_t client_len = sizeof(client_addr);

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

    Connection conn;
    conn.state = LISTEN;

    while (1) {
        ssize_t bytes = recvfrom(
            sockfd,
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

        conn.peer_addr = client_addr;
        conn.peer_len = client_len;
        conn.sockfd = sockfd;

        handle_packet(&conn, buffer, bytes, sockfd);

        printf("Received from %s:%d: %s\n",
            inet_ntoa(client_addr.sin_addr),
            ntohs(client_addr.sin_port),
            buffer
        );
    }

    close(sockfd);
    return 0;
}

void handle_packet(Connection* conn, const uint8_t* buf, size_t buf_len, int sockfd) {
    Packet packet;
    if (packet_decode(buf, buf_len, &packet) != 0) {
        fprintf(stderr, "invalid packet (bad length or checksum)\n");
        return;
    }

    conn->snd_ack = packet.ack_number;
    if (packet.flags & FLAG_SYN || packet.flags & FLAG_FIN) {
        conn->rcv_seq = packet.seq_number + 1;
    } else if (packet.data_len > 0) {
        conn->rcv_seq = packet.seq_number + packet.data_len;
        memcpy(conn->rcv_buf, packet.data, packet.data_len);
        conn->rcv_len = packet.data_len;
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

    fsm_dispatch(conn, event);

}
