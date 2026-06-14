#include <netinet/in.h>
#include <stdint.h>
#include <stddef.h>

#define PACKET_DATA_SIZE 512

#define FLAG_SYN 0x0001
#define FLAG_ACK 0x0002
#define FLAG_FIN 0x0004
#define FLAG_RST 0x0008
#define FLAG_PSH 0x0010

typedef struct {
    uint32_t seq_number;
    uint32_t ack_number;
    uint16_t flags;
    uint16_t window_size;
    uint16_t data_len;
    uint16_t checksum;
    uint8_t data[PACKET_DATA_SIZE];
} Packet;

#define PACKET_HEADER_SIZE offsetof(Packet, data)

uint16_t packet_checksum(const uint8_t* buf, size_t len);

int packet_verify_checksum(const uint8_t* buf);

int packet_decode(const uint8_t* buf, ssize_t buf_len, Packet* out);

int packet_encode(const Packet* in, uint8_t* buf, size_t buf_len);
