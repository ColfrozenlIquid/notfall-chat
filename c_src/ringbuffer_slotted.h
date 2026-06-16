#pragma once
#include <pthread.h>
#include <sched.h>

#define RB_SLOTS_CAPACITY 64

typedef struct {
    void* slots[RB_SLOTS_CAPACITY];
    size_t item_size;
    size_t head;
    size_t tail;
    size_t count;
    pthread_mutex_t mutex;
    pthread_cond_t not_empty;
} RingBufferSlotted;

void rbs_init(RingBufferSlotted* rb, size_t item_size);

int rbs_push(RingBufferSlotted* rb, const void* item);

int rbs_pop(RingBufferSlotted* rb, void* dst);

int rbs_try_pop(RingBufferSlotted* rb, void* dst);

size_t rbs_count(RingBufferSlotted* rb);
