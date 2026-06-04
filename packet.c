#include <bits/types/struct_iovec.h>
#include <netinet/in.h>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>
#include <arpa/inet.h>

#include "packet.h"

uint16_t packet_checksum(const char* buf, size_t len) {
    uint32_t sum = 0;
    const uint16_t* ptr = (const uint16_t*)buf;

    while(len > 1) {
        sum += *ptr++;
        len -= 2;
    }
    if (len == 1) {
        sum += *(const uint8_t*)ptr;
    }

    while (sum >> 16) {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    return (uint16_t)~sum;
}

int packet_verify_checksum(const char* buf) {
    const Packet* p = (const Packet*)buf;
    size_t len = sizeof(Packet) - PACKET_DATA_SIZE + ntohs(p->data_len);
    return packet_checksum(buf, len) == 0xFFFF ? 0 : -1;
}

int packet_decode(const char* buf, ssize_t buf_len, Packet* out) {
    if (buf_len < (ssize_t)PACKET_HEADER_SIZE) return -1;

    uint16_t data_len = ntohs(((const Packet*)buf)->data_len);
    if (data_len > PACKET_DATA_SIZE) return -1;
    if (buf_len < (ssize_t)(PACKET_HEADER_SIZE + data_len)) return -1;

    if (packet_verify_checksum(buf) != 0) return -1;

    memcpy(out, buf, PACKET_HEADER_SIZE + data_len);

    out->seq_number = ntohl(out->seq_number);
    out->ack_number = ntohl(out->ack_number);
    out->flags = ntohs(out->flags);
    out->window_size = ntohs(out->window_size);
    out->data_len = ntohs(out->data_len);
    out->checksum = ntohs(out->checksum);

    if (out->data_len > PACKET_DATA_SIZE) {
        return -1;
    }

    return 0;
}

int packet_encode(const Packet* in, char* buf, size_t buf_len) {
    size_t total_size = PACKET_HEADER_SIZE + in->data_len;

    if (in->data_len > PACKET_DATA_SIZE) return -1;
    if (buf_len < total_size) return -1;

    Packet wire;
    wire.seq_number = htonl(in->seq_number);
    wire.ack_number = htonl(in->ack_number);
    wire.flags = htons(in->flags);
    wire.window_size = htons(in->window_size);
    wire.data_len = htons(in->data_len);
    wire.checksum = 0;

    memcpy(wire.data, in->data, in->data_len);

    uint16_t checksum = packet_checksum((const char*)&wire, total_size);
    wire.checksum = htons(checksum);

    memcpy(buf, &wire, total_size);

    return (int)total_size;
}
