#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include "client.h"

// Client
int main(int argc, char* argv[]) {
    if (argc != 3) {
        fprintf(stderr, "Usage: %s <server_ip> <port>\n", argv[0]);
        return 1;
    }
    int port = atoi(argv[2]);
    if (port <= 0 || port > 65535) {
        fprintf(stderr, "Invalid port number: %s\n", argv[2]);
        return 1;
    }
    Connection conn;
    connect_to_server(argv[1], port, &conn);

    while(1) {
        int len = 0;
        uint8_t* input = read_input(&len);
        if(!input) break;
        write_to_connection(&conn, input, len);
        free(input);
    }

    pthread_join(conn.recv_tid, NULL);
    pthread_join(conn.send_tid, NULL);

    return 0;
}
