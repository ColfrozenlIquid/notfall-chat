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
IFACE      ?= wlan0
CLIENT_IP  ?= 192.168.50.2
NETMASK    ?= 24
SSID       ?= MyAdHocNet
WPA_CONF   = /tmp/wpa_adhoc.conf
WPA_PID    = /tmp/wpa_adhoc.pid

.PHONY: up down check

## Connect to the AP
up:
	@echo "[CLIENT] Disabling NetworkManager on $(IFACE)..."
	nmcli device set $(IFACE) managed no

	@echo "[CLIENT] Bringing interface down..."
	ip link set $(IFACE) down

	@echo "[CLIENT] Writing wpa_supplicant config..."
	@printf 'ctrl_interface=/run/wpa_supplicant\nnetwork={\n\tssid="%s"\n\tkey_mgmt=NONE\n}\n' "$(SSID)" > $(WPA_CONF)

	@echo "[CLIENT] Bringing interface up..."
	ip link set $(IFACE) up

	@echo "[CLIENT] Starting wpa_supplicant..."
	wpa_supplicant -B -i $(IFACE) -c $(WPA_CONF) -P $(WPA_PID)

	@echo "[CLIENT] Waiting for association..."
	@for i in 1 2 3 4 5 6 7 8 9 10; do \
		STATUS=$$(iw dev $(IFACE) link | grep "Connected to"); \
		if [ -n "$$STATUS" ]; then \
			echo "[CLIENT] Associated: $$STATUS"; \
			break; \
		fi; \
		echo "[CLIENT] Waiting... ($$i/10)"; \
		sleep 1; \
	done; \
	STATUS=$$(iw dev $(IFACE) link | grep "Connected to"); \
	if [ -z "$$STATUS" ]; then \
		echo "[CLIENT] ERROR: Failed to associate after 10s. Run 'make check' to debug."; \
		exit 1; \
	fi

	@echo "[CLIENT] Assigning IP $(CLIENT_IP)/$(NETMASK)..."
	ip addr add $(CLIENT_IP)/$(NETMASK) dev $(IFACE)

	@echo "[CLIENT] Done. Interface info:"
	iw dev $(IFACE) info

## Tear everything down
down:
	@echo "[CLIENT] Stopping wpa_supplicant..."
	-kill $$(cat $(WPA_PID) 2>/dev/null) 2>/dev/null || true
	-rm -f $(WPA_CONF) $(WPA_PID)

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
	@echo ""
	@echo "=== wpa_supplicant running? ==="
	@cat $(WPA_PID) 2>/dev/null | xargs -I{} ps -p {} -o pid,comm || echo "wpa_supplicant not running"
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
