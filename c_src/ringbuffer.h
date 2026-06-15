#pragma once
#include <bits/types/struct_iovec.h>
#include <stdint.h>

#define RING_BUFFER_SIZE 8192   // 8KB

typedef struct {
    uint8_t data[RING_BUFFER_SIZE];
    size_t head;
    size_t tail;
    size_t count;
} RingBuffer;

void rb_init(RingBuffer* rb);

void rb_write(RingBuffer* rb, const uint8_t* data, size_t len);

size_t rb_free_space(RingBuffer* rb);

void rb_consume(RingBuffer* rb, uint8_t* dst);

uint32_t rb_peek_len(RingBuffer* rb);
