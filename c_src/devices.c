#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>
#include <arpa/inet.h>
#include "ifaddrs.h"
#include <sys/types.h>
#include <sys/ioctl.h>
#include <net/if.h>

#include "devices.h"

size_t get_network_devices(NetworkDevice* devices, size_t len) {
    struct ifaddrs* ifaddr;
    struct ifaddrs* ifa;
    char addr_str[INET6_ADDRSTRLEN];

    if (getifaddrs(&ifaddr) == -1) {
        perror("getifaddrs");
        return 1;
    }

    size_t count = 0;
    for (ifa = ifaddr; ifa != NULL; ifa = ifa->ifa_next) {
        if (ifa->ifa_addr == NULL) continue;

        if (strncmp(ifa->ifa_name, "en", 2) != 0 &&
            strncmp(ifa->ifa_name, "wl", 2) != 0) continue;

        int family = ifa->ifa_addr->sa_family;
        if (family != AF_INET && family != AF_INET6) continue;
        if (count >= len) break;
        void* addr_ptr;

        if (family == AF_INET) {
            addr_ptr = &((struct sockaddr_in*)ifa->ifa_addr)->sin_addr;
            devices[count].is_ipv6 = 0;
        } else {
            addr_ptr = &((struct sockaddr_in6*)ifa->ifa_addr)->sin6_addr;
            devices[count].is_ipv6 = 1;
        }

        inet_ntop(family, addr_ptr, devices[count].addr, sizeof(devices[count].addr));
        strncpy(devices[count].name, ifa->ifa_name, DEVICE_NAME_LEN - 1);
        devices[count].name[DEVICE_NAME_LEN-1] = '\0';

        count++;
    }
    freeifaddrs(ifaddr);

    return count;
}
