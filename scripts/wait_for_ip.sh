#!/bin/bash
# Waits until the given interface has an IP assigned (via DHCP)
IFACE=$1
for i in $(seq 1 30); do
    IP=$(ip -4 addr show "$IFACE" | grep -oP '(?<=inet )[\d.]+')
    if [ -n "$IP" ]; then
        echo "$IP"
        exit 0
    fi
    sleep 1
done
echo "ERROR: no IP on $IFACE after timeout" >&2
exit 1
