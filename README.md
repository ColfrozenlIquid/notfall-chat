1. iw list | grep -A 10 "Supported interface modes"
2. sudo nmcli device set `wlan0` managed no
2. sudo ip link set `wlan0` down
3. sudo iw dev `wlan0` set type ibss
4. sudo ip link set `wlan0` up
5. sudo iw dev wlan0 ibss join MyAdHocNet 2412
6. sudo ip addr add 192.168.50.1/24 dev `wlan0`
7. sudo ip addr add 192.168.50.2/24 dev `wlan0`
8. ping 192.168.50.2
