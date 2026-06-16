#pragma once
#include <arpa/inet.h>
#include <net/if.h>
#include <netinet/in.h>

#define DEVICE_NAME_LEN IFNAMSIZ  // IFNAMESIZ = 16
#define DEVICE_ADDR_LEN INET6_ADDRSTRLEN // INET6_ADDRSTRLEN = 46

typedef struct {
    char name[DEVICE_NAME_LEN];
    char addr[DEVICE_ADDR_LEN];
    int is_ipv6;    // 0 = IPv4, 1 = IPv6
} NetworkDevice;

size_t get_network_devices(NetworkDevice* devices, size_t len);
