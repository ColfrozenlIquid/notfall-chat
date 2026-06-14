#include "discovery_listener.h"
#include <pthread.h>
#include <stdio.h>
#include <time.h>

static DiscoveryQueue* g_queue = NULL;
static pthread_t g_thread;

static void* listener_thread(void* arg) {
    printf("listener thread started\n");
    fflush(stdout);
    listen_discovery(discovery_queue_callback, g_queue);
    return NULL;
}

void discovery_listener_start() {
    g_queue = malloc(sizeof(DiscoveryQueue));
    *g_queue = discovery_queue_create();
    pthread_create(&g_thread, NULL, listener_thread, NULL);
}

int discovery_listener_pop(DiscoveredPeer* out) {
    return discovery_queue_pop(g_queue, out);
}
