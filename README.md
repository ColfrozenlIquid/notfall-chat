On `Host`:

```shell
nmcli device wifi hotspot ifname `wlp13s0` ssid mylink password "password123"

nmcli connection modify Hotspot ipv4.addresses 192.168.4.1/24 ipv4.method shared

nmcli connection up Hotspot
```

On `Client`:

Connect to Hotspot with "password123"
