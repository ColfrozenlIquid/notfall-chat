IFACE      ?= wlan0
CLIENT_IP  ?= 192.168.50.2
NETMASK    ?= 24
SSID       ?= MyAdHocNet

.PHONY: up down check

## Connect to the AP
up:
	@echo "[CLIENT] Disabling NetworkManager on $(IFACE)..."
	nmcli device set $(IFACE) managed no

	@echo "[CLIENT] Bringing interface down..."
	ip link set $(IFACE) down

	@echo "[CLIENT] Bringing interface up..."
	ip link set $(IFACE) up

	@echo "[CLIENT] Connecting to SSID: $(SSID)..."
	iw dev $(IFACE) connect $(SSID)

	@echo "[CLIENT] Waiting for association..."
	@for i in 1 2 3 4 5; do \
		STATUS=$$(iw dev $(IFACE) link | grep "Connected to"); \
		if [ -n "$$STATUS" ]; then \
			echo "[CLIENT] Associated: $$STATUS"; \
			break; \
		fi; \
		echo "[CLIENT] Waiting... ($$i/5)"; \
		sleep 1; \
	done

	@echo "[CLIENT] Assigning IP $(CLIENT_IP)/$(NETMASK)..."
	ip addr add $(CLIENT_IP)/$(NETMASK) dev $(IFACE)

	@echo "[CLIENT] Done. Interface info:"
	iw dev $(IFACE) info

## Tear everything down
down:
	@echo "[CLIENT] Disconnecting from SSID..."
	-iw dev $(IFACE) disconnect 2>/dev/null || true

	@echo "[CLIENT] Removing IP address..."
	-ip addr del $(CLIENT_IP)/$(NETMASK) dev $(IFACE) 2>/dev/null || true

	@echo "[CLIENT] Bringing interface down..."
	-ip link set $(IFACE) down

	@echo "[CLIENT] Returning interface to NetworkManager..."
	-nmcli device set $(IFACE) managed yes

	@echo "[CLIENT] Done. Interface returned to managed mode."

## Show current status
check:
	@echo "=== Interface info ==="
	iw dev $(IFACE) info
	@echo ""
	@echo "=== IP addresses ==="
	ip addr show $(IFACE)
	@echo ""
	@echo "=== Association status ==="
	iw dev $(IFACE) link
