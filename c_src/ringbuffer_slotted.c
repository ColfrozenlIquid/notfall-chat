#include "ringbuffer_slotted.h"
#include <bits/pthreadtypes.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <pthread.h>
#include <time.h>

void rbs_init(RingBufferSlotted* rb, size_t item_size) {
    rb->item_size = item_size;
    rb->head = 0;
    rb->tail = 0;
    rb->count = 0;
    pthread_mutex_init(&rb->mutex, NULL);
    pthread_cond_init(&rb->not_empty, NULL);
}

int rbs_push(RingBufferSlotted* rb, const void* item) {
    pthread_mutex_lock(&rb->mutex);

    if (rb->count == RB_SLOTS_CAPACITY) {
        fprintf(stderr, "rbs_push: buffer full\n");
        pthread_mutex_unlock(&rb->mutex);
        return -1;
    }

    void* slot = malloc(rb->item_size);
    if (!slot) {
        pthread_mutex_unlock(&rb->mutex);
        return -1;
    }

    memcpy(slot, item, rb->item_size);

    rb->slots[rb->head] = slot;
    rb->head = (rb->head + 1) % RB_SLOTS_CAPACITY;
    rb->count++;

    pthread_cond_signal(&rb->not_empty);
    pthread_mutex_unlock(&rb->mutex);
    return 0;
}

int rbs_pop(RingBufferSlotted* rb, void* dst) {
    pthread_mutex_lock(&rb->mutex);

    while (rb->count == 0) {
        pthread_cond_wait(&rb->not_empty, &rb->mutex);
    }

    memcpy(dst, rb->slots[rb->tail], rb->item_size);
    free(rb->slots[rb->tail]);
    rb->slots[rb->tail] = NULL;
    rb->tail = (rb->tail + 1) % RB_SLOTS_CAPACITY;
    rb->count--;

    pthread_mutex_unlock(&rb->mutex);
    return 0;
}

int rbs_try_pop(RingBufferSlotted* rb, void* dst) {
    pthread_mutex_lock(&rb->mutex);

    if (rb->count == 0) {
        pthread_mutex_unlock(&rb->mutex);
        return -1;
    }

    memcpy(dst, rb->slots[rb->tail], rb->item_size);
    free(rb->slots[rb->tail]);
    rb->slots[rb->tail] = NULL;
    rb->tail = (rb->tail + 1) % RB_SLOTS_CAPACITY;
    rb->count--;

    pthread_mutex_unlock(&rb->mutex);
    return 0;
}

size_t rbs_count(RingBufferSlotted* rb) {
    pthread_mutex_lock(&rb->mutex);
    size_t c = rb->count;
    pthread_mutex_unlock(&rb->mutex);
    return c;
}
