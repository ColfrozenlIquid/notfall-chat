#pragma once
#include "ringbuffer_slotted.h"
#include <bits/types/struct_iovec.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdint.h>
#include <netinet/in.h>
#include <stdint.h>
#include <stddef.h>

#define DISCOVERY_NAME_LEN 32
#define DISCOVERY_MAGIC 0x44534356 // "DSCV"
#define BROADCAST_PORT 5005
#define BROADCAST_ADDR "255.255.255.255"
#define DISCOVER_INTERVAL_SEC 5

enum {
    DISCOVERY_MAGIC_SIZE = 4,
    DISCOVERY_TIMESTAMP_SIZE = 8,
    DISCOVERY_PORT_SIZE = 2,

    DISCOVERY_HEADER_SIZE =
        DISCOVERY_MAGIC_SIZE +
        DISCOVERY_TIMESTAMP_SIZE +
        DISCOVERY_PORT_SIZE,

    DISCOVERY_PACKET_SIZE =
        DISCOVERY_HEADER_SIZE +
        DISCOVERY_NAME_LEN,

    DISCOVERED_PEER_SIZE =
        DISCOVERY_PACKET_SIZE +
        INET_ADDRSTRLEN,
};

typedef struct {
    RingBufferSlotted rbs;
} DiscoveryQueue;

typedef struct {
    uint32_t magic_bytes;
    uint64_t timestamp;
    uint16_t port;
    uint8_t name[DISCOVERY_NAME_LEN];
} DiscoveryPacket;

typedef struct {
    uint64_t timestamp;
    uint16_t port;
    char sender_ip[INET_ADDRSTRLEN];
    uint8_t name[DISCOVERY_NAME_LEN];
} DiscoveredPeer;

DiscoveryQueue discovery_queue_create();

typedef void (*discovery_callback)(DiscoveredPeer* peer, void* userdata);

DiscoveryPacket discovery_packet_create();

int broadcast_discovery(uint8_t* name, size_t name_len, uint16_t tcp_port);

int listen_discovery(discovery_callback cb, void* userdata);

size_t discovery_packet_encode(const DiscoveryPacket* pkt, uint8_t* buf);

int discovery_packet_decode(DiscoveryPacket* pkt, const uint8_t* buf, size_t len);

void discovery_queue_callback(DiscoveredPeer* peer, void* userdata);

int discovery_queue_pop(DiscoveryQueue* q, DiscoveredPeer* out);

static uint64_t htonll(uint64_t value) {
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    return ((uint64_t)htonl(value & 0xFFFFFFFFULL) << 32) | htonl(value >> 32);
#else
    return value;
#endif
}

static uint64_t ntohll(uint64_t value) {
#if __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
    return ((uint64_t)ntohl(value & 0xFFFFFFFFULL) << 32) |
        ntohl(value >> 32);
#else
    return value;
#endif
}
