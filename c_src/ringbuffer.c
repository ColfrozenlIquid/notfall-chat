#include "ringbuffer.h"
#include <bits/types/struct_iovec.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void rb_init(RingBuffer* rb) {
    rb->head = 0;
    rb->tail = 0;
    rb->count = 0;
}

void rb_write(RingBuffer* rb, const uint8_t* data, size_t len) {
    if (rb_free_space(rb) < len) {
        fprintf(stderr, "Ring buffer is full. This should never happen. Increase size of ring buffer.\n");
        exit(EXIT_FAILURE);
    }

    for (size_t i = 0; i < len; i++) {
        rb->data[rb->head] = data[i];
        rb->head = (rb->head + 1) % RING_BUFFER_SIZE;
    }
    rb->count += len;
}

// 1. Read the first 4 bytes to get the prefixed message length
// 2. Check if the message can be read in one contigous chunk
// 3. Copy message to destination buffer
// 4. Decrement count and move tail position forward
void rb_consume(RingBuffer* rb, uint8_t* dst) {
    uint32_t data_len = ((uint32_t)rb->data[rb->tail] << 24)
        |   ((uint32_t)rb->data[(rb->tail + 1) % RING_BUFFER_SIZE] << 16)
        |   ((uint32_t)rb->data[(rb->tail + 2) % RING_BUFFER_SIZE] << 8)
        |   ((uint32_t)rb->data[(rb->tail + 3) % RING_BUFFER_SIZE]);

    size_t total = 4 + data_len;          // header + payload
    size_t start = rb->tail % RING_BUFFER_SIZE;
    size_t first_chunk = RING_BUFFER_SIZE - start;
    if (first_chunk > total) {
        first_chunk = total;
    }

    memcpy(dst, rb->data + start, first_chunk);
    if (total > first_chunk) {
        memcpy(dst + first_chunk, rb->data, total - first_chunk);
    }

    printf("Dst buffer\n");
    for (int i = 0; i < total; i++) {
        printf("[%hhu]", dst[i]);
    }

    rb->tail = (rb->tail + total) % RING_BUFFER_SIZE;
    rb->count -= total;
}

size_t rb_free_space(RingBuffer* rb) {
    return RING_BUFFER_SIZE - rb->count;
}

uint32_t rb_peek_len(RingBuffer* rb) {
    return ((uint32_t)rb->data[rb->tail] << 24)
         | ((uint32_t)rb->data[(rb->tail + 1) % RING_BUFFER_SIZE] << 16)
         | ((uint32_t)rb->data[(rb->tail + 2) % RING_BUFFER_SIZE] << 8)
         | ((uint32_t)rb->data[(rb->tail + 3) % RING_BUFFER_SIZE]);
}
