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

int run_server() {

    int sockfd;
    char buffer[BUFFER_SIZE];

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
    server_addr.sin_port = htons(PORT);

    if (bind(sockfd, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        perror("bind socket");
        close(sockfd);
        return 1;
    }

    printf("UDP server socket listening on port: %d\n", PORT);

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

void handle_packet(Connection* conn, const char* buf, size_t buf_len, int sockfd) {
    Packet packet;
    if (packet_decode(buf, buf_len, &packet) != 0) {
        fprintf(stderr, "invalid packet (bad length or checksum)\n");
        return;
    }

    conn->snd_ack = packet.ack_number;
}
