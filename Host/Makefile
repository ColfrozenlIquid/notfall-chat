IFACE       ?= wlp13s0
AP_IP       ?= 192.168.50.1
NETMASK     ?= 24
SSID        ?= MyAdHocNet
CHANNEL     ?= 6
HOSTAPD_CONF = /tmp/hostapd_adhoc.conf
HOSTAPD_PID  = /tmp/hostapd_adhoc.pid

.PHONY: up down check

## Bring up the AP
up: $(HOSTAPD_CONF)
	@echo "[AP] Disabling NetworkManager on $(IFACE)..."
	nmcli device set $(IFACE) managed no

	@echo "[AP] Bringing interface down..."
	ip link set $(IFACE) down

	@echo "[AP] Bringing interface up..."
	ip link set $(IFACE) up

	@echo "[AP] Starting hostapd..."
	hostapd -B -P $(HOSTAPD_PID) $(HOSTAPD_CONF)

	@echo "[AP] Assigning IP $(AP_IP)/$(NETMASK)..."
	ip addr add $(AP_IP)/$(NETMASK) dev $(IFACE)

	@echo "[AP] Done. Interface info:"
	iw dev $(IFACE) info

## Generate hostapd config
$(HOSTAPD_CONF):
	@echo "[AP] Writing hostapd config to $(HOSTAPD_CONF)..."
	@printf 'interface=%s\n'        "$(IFACE)"      > $(HOSTAPD_CONF)
	@printf 'ssid=%s\n'             "$(SSID)"       >> $(HOSTAPD_CONF)
	@printf 'hw_mode=g\n'                           >> $(HOSTAPD_CONF)
	@printf 'channel=%s\n'          "$(CHANNEL)"    >> $(HOSTAPD_CONF)
	@printf 'auth_algs=1\n'                         >> $(HOSTAPD_CONF)
	@printf 'wmm_enabled=0\n'                       >> $(HOSTAPD_CONF)

## Tear everything down
down:
	@echo "[AP] Stopping hostapd..."
	-kill $$(cat $(HOSTAPD_PID) 2>/dev/null) 2>/dev/null || true
	-rm -f $(HOSTAPD_PID) $(HOSTAPD_CONF)

	@echo "[AP] Removing IP address..."
	-ip addr del $(AP_IP)/$(NETMASK) dev $(IFACE) 2>/dev/null || true

	@echo "[AP] Bringing interface down..."
	-ip link set $(IFACE) down

	@echo "[AP] Returning interface to NetworkManager..."
	-nmcli device set $(IFACE) managed yes

	@echo "[AP] Done. Interface returned to managed mode."

## Show current status
check:
	@echo "=== Interface info ==="
	iw dev $(IFACE) info
	@echo ""
	@echo "=== IP addresses ==="
	ip addr show $(IFACE)
	@echo ""
	@echo "=== hostapd running? ==="
	@cat $(HOSTAPD_PID) 2>/dev/null | xargs -I{} ps -p {} -o pid,comm || echo "hostapd not running"
