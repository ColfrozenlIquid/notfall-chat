#include <bits/types/struct_iovec.h>
#include <netinet/in.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>
#include <arpa/inet.h>

#include "packet.h"

uint16_t packet_checksum(const uint8_t* buf, size_t len) {
    uint32_t sum = 0;

    while (len > 1) {
        uint16_t word = ((uint16_t)buf[0] << 8) | buf[1];
        sum += word;
        buf += 2;
        len -= 2;
    }

    if (len == 1)
        sum += ((uint16_t)buf[0] << 8);

    while (sum >> 16)
        sum = (sum & 0xFFFF) + (sum >> 16);

    return (uint16_t)~sum;
}

int packet_verify_checksum(const uint8_t* buf) {
    const Packet* p = (const Packet*)buf;
    size_t len = PACKET_HEADER_SIZE + ntohs(p->data_len);
    uint16_t result = packet_checksum(buf, len);
    return result == 0 ? 0 : -1;
}

int packet_decode(const uint8_t* buf, ssize_t buf_len, Packet* out) {
    fprintf(stderr, "sizeof(Packet)=%zu, PACKET_DATA_SIZE=%u, PACKET_HEADER_SIZE=%zu, diff=%zu\n",
            sizeof(Packet), PACKET_DATA_SIZE, PACKET_HEADER_SIZE,
            sizeof(Packet) - PACKET_DATA_SIZE);

    if (buf_len < (ssize_t)PACKET_HEADER_SIZE) {
        fprintf(stderr, "packet_decode: too short — got %zd, need %zu\n",
                buf_len, PACKET_HEADER_SIZE);
        return -1;
    }

    uint16_t data_len = ntohs(((const Packet*)buf)->data_len);

    if (data_len > PACKET_DATA_SIZE) {
        fprintf(stderr, "packet_decode: data_len %u exceeds max %u\n",
                data_len, PACKET_DATA_SIZE);
        return -1;
    }

    if (buf_len < (ssize_t)(PACKET_HEADER_SIZE + data_len)) {
        fprintf(stderr, "packet_decode: buffer too short for data — got %zd, need %zu\n",
                buf_len, PACKET_HEADER_SIZE + data_len);
        return -1;
    }

    if (packet_verify_checksum(buf) != 0) {
        const Packet* raw = (const Packet*)buf;
        fprintf(stderr, "packet_decode: checksum failed — "
                "seq=%u ack=%u flags=0x%04x data_len=%u stored_checksum=0x%04x\n",
                ntohl(raw->seq_number),
                ntohl(raw->ack_number),
                ntohs(raw->flags),
                data_len,
                ntohs(raw->checksum));
        return -1;
    }

    memcpy(out, buf, PACKET_HEADER_SIZE + data_len);
    out->seq_number  = ntohl(out->seq_number);
    out->ack_number  = ntohl(out->ack_number);
    out->flags       = ntohs(out->flags);
    out->window_size = ntohs(out->window_size);
    out->data_len    = ntohs(out->data_len);
    out->checksum    = ntohs(out->checksum);

    return 0;
}

int packet_encode(const Packet* in, uint8_t* buf, size_t buf_len) {
    size_t total_size = PACKET_HEADER_SIZE + in->data_len;

    if (in->data_len > PACKET_DATA_SIZE) return -1;
    if (buf_len < total_size) return -1;

    Packet wire;
    memset(&wire, 0, sizeof(wire));
    wire.seq_number = htonl(in->seq_number);
    wire.ack_number = htonl(in->ack_number);
    wire.flags = htons(in->flags);
    wire.window_size = htons(in->window_size);
    wire.data_len = htons(in->data_len);
    wire.checksum = 0;

    memcpy(wire.data, in->data, in->data_len);

    uint16_t checksum = packet_checksum((const uint8_t*)&wire, total_size);
    wire.checksum = htons(checksum);

    memcpy(buf, &wire, total_size);

    return (int)total_size;
}
