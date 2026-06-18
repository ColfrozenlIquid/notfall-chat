#!/bin/bash
# Waits for and prints the p2p-* interface created by wpa_supplicant
for i in $(seq 1 20); do
    IFACE=$(ip link show | grep -oP 'p2p-\S+' | tr -d ':' | head -1)
    if [ -n "$IFACE" ]; then
        echo "$IFACE"
        exit 0
    fi
    sleep 0.5
done
echo "ERROR: no p2p interface found" >&2
exit 1
