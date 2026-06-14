#include <arpa/inet.h>
#include <asm-generic/socket.h>
#include <bits/types/struct_iovec.h>
#include <errno.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#include "discovery.h"
#include "ringbuffer_slotted.h"

DiscoveryPacket discovery_packet_create() {
    DiscoveryPacket pkt = {0};
    pkt.magic_bytes = DISCOVERY_MAGIC;
    return pkt;
}

DiscoveryQueue discovery_queue_create() {
    DiscoveryQueue queue = {};
    RingBufferSlotted rbs;
    rbs_init(&rbs, DISCOVERED_PEER_SIZE);
    queue.rbs = rbs;
    return queue;
}

void discovery_queue_callback(DiscoveredPeer* peer, void* userdata) {
    DiscoveryQueue* queue = (DiscoveryQueue*)userdata;
    rbs_push(&queue->rbs, peer);
}

int discovery_queue_pop(DiscoveryQueue* q, DiscoveredPeer* out) {
    return rbs_try_pop(&q->rbs, out);
}

int broadcast_discovery(uint8_t* name, size_t name_len, uint16_t tcp_port) {
    int sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd < 0) {
        perror("socket error");
        return -1;
    }

    int broadcast_opt = 1;
    if (setsockopt(sockfd, SOL_SOCKET, SO_BROADCAST, &broadcast_opt, sizeof(broadcast_opt)) < 0) {
        int err = errno;  // capture immediately
        fprintf(stderr, "setsockopt SO_BROADCAST: %s (errno=%d)\n", strerror(err), err);
        close(sockfd);
        return -1;
    }

    struct sockaddr_in broadcast_addr = {0};
    broadcast_addr.sin_family = AF_INET;
    broadcast_addr.sin_port = htons(BROADCAST_PORT);
    broadcast_addr.sin_addr.s_addr = inet_addr(BROADCAST_ADDR);

    DiscoveryPacket pkt = discovery_packet_create();
    pkt.magic_bytes = DISCOVERY_MAGIC;
    pkt.timestamp = (uint64_t)time(NULL);
    pkt.port = tcp_port;
    memcpy(pkt.name, name, name_len);

    uint8_t buf[DISCOVERY_PACKET_SIZE];

    size_t len = discovery_packet_encode(&pkt, buf);

    while(1) {
        sendto(sockfd, buf, len, 0, (struct sockaddr*)&broadcast_addr, sizeof(broadcast_addr));
        sleep(DISCOVER_INTERVAL_SEC);
    }

    return 0;
}

int listen_discovery(discovery_callback cb, void* userdata) {
    int sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd < 0) {
        perror("socket error");
        return -1;
    }

    int reuse = 1;
    if (setsockopt(sockfd, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse)) < 0) {
        perror("setsockopt SO_REUSEADDR");
        close(sockfd);
        return -1;
    }

    struct sockaddr_in bind_addr = {0};
    bind_addr.sin_family = AF_INET;
    bind_addr.sin_port = htons(BROADCAST_PORT);
    bind_addr.sin_addr.s_addr = htonl(INADDR_ANY);

    if (bind(sockfd, (struct sockaddr*)&bind_addr, sizeof(bind_addr)) < 0) {
        perror("bind error");
        close(sockfd);
        return -1;
    }

    while (1) {
        struct sockaddr_in sender = {0};
        socklen_t sender_len = sizeof(sender);

        uint8_t buf[DISCOVERY_PACKET_SIZE];

        ssize_t n = recvfrom(sockfd, buf, sizeof(buf), 0, (struct sockaddr*)&sender, &sender_len);

        if (n < 0) {
            perror("recvfrom error");
            continue;
        }

        DiscoveryPacket pkt;

        if (discovery_packet_decode(&pkt, buf, n)) {
            continue;
        }

        if (pkt.magic_bytes != DISCOVERY_MAGIC) {
            continue;
        }

        DiscoveredPeer peer;
        memcpy(peer.name, pkt.name, DISCOVERY_NAME_LEN);
        peer.port = pkt.port;
        peer.timestamp = pkt.timestamp;
        inet_ntop(AF_INET, &sender.sin_addr, peer.sender_ip, INET_ADDRSTRLEN);

        cb(&peer, userdata);
    }

    close(sockfd);
    return 0;
}

size_t discovery_packet_encode(const DiscoveryPacket* pkt, uint8_t* buf) {
    uint8_t* ptr = buf;

    uint32_t magic = htonl(pkt->magic_bytes);
    uint64_t timestamp = htonll(pkt->timestamp);
    uint16_t port = htons(pkt->port);

    memcpy(ptr, &magic, sizeof(magic));
    ptr += sizeof(magic);

    memcpy(ptr, &timestamp, sizeof(timestamp));
    ptr += sizeof(timestamp);

    memcpy(ptr, &port, sizeof(port));
    ptr += sizeof(port);

    memcpy(ptr, &pkt->name, DISCOVERY_NAME_LEN);
    ptr += DISCOVERY_NAME_LEN;

    return (size_t)(ptr - buf);
}

int discovery_packet_decode(DiscoveryPacket* pkt, const uint8_t* buf, size_t len) {
    if (len < DISCOVERY_HEADER_SIZE + DISCOVERY_NAME_LEN) {
        return -1;
    }

    const uint8_t* ptr = buf;
    uint32_t magic;
    uint64_t timestamp;
    uint16_t port;

    memcpy(&magic, ptr, sizeof(magic));
    ptr += sizeof(magic);

    memcpy(&timestamp, ptr, sizeof(timestamp));
    ptr += sizeof(timestamp);

    memcpy(&port, ptr, sizeof(port));
    ptr += sizeof(port);

    memcpy(pkt->name, ptr, DISCOVERY_NAME_LEN);
    ptr += DISCOVERY_NAME_LEN;

    pkt->magic_bytes = ntohl(magic);
    pkt->timestamp = ntohll(timestamp);
    pkt->port = ntohs(port);

    return 0;
}
