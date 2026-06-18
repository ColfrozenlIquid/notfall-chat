build: clean
	mkdir -p build && cd build && cmake .. && make

run: build
	./build/server

run-client:
	./build/client 127.0.0.1 12345

clean:
	rm -rf build

build-raspberry:
	cargo build --release --target aarch64-unknown-linux-gnu
	scp target/aarch64-unknown-linux-gnu/release/notfall-chat pi@192.168.1.135:~/

# ─────────────────────────────────────────────
#  Configuration
# ─────────────────────────────────────────────
PHYS_IFACE   ?= wlan0
GO_IP        := 192.168.42.1
GO_CIDR      := $(GO_IP)/24
BCAST        := 192.168.42.255
APP          ?= ./your_p2p_app

WPA_PID      := /var/run/wpa_supplicant_p2p.pid
WPA_CTRL     := /var/run/wpa_supplicant
DNSMASQ_PID  := /var/run/dnsmasq_p2p.pid
P2P_IFACE_F  := /tmp/p2p_iface_name     # cached interface name

# ─────────────────────────────────────────────
#  Phony targets
# ─────────────────────────────────────────────
.PHONY: all go client status stop clean \
        _wpa-go _wpa-client _p2p-iface \
        _ip-go _dhcp _ip-client app check deps

all:
	@echo "Targets:"
	@echo "  make go       — set up this node as P2P Group Owner"
	@echo "  make client   — set up this node as P2P Client"
	@echo "  make app      — launch the application"
	@echo "  make status   — show network status"
	@echo "  make stop     — tear down everything"
	@echo "  make clean    — stop + remove temp files"

# ─────────────────────────────────────────────
#  Dependency check
# ─────────────────────────────────────────────
deps:
	@echo "[check] verifying required tools..."
	@for cmd in wpa_supplicant wpa_cli iw dnsmasq dhclient ip; do \
	    command -v $$cmd >/dev/null 2>&1 \
	        && echo "  OK  $$cmd" \
	        || echo "  MISSING  $$cmd — install with: apt install $$cmd"; \
	done
	@iw list 2>/dev/null | grep -q "P2P-GO" \
	    && echo "  OK  driver supports P2P-GO" \
	    || echo "  WARN  P2P-GO not listed — may still work, check iw list"

# ─────────────────────────────────────────────
#  GO setup (run on the one designated GO node)
# ─────────────────────────────────────────────
go: deps _kill-wpa _wpa-go _p2p-iface _ip-go _dhcp
	@echo ""
	@echo "══════════════════════════════════════"
	@echo "  P2P Group Owner ready"
	@echo "  Interface : $$(cat $(P2P_IFACE_F))"
	@echo "  IP        : $(GO_IP)"
	@echo "  Broadcast : $(BCAST)"
	@echo "══════════════════════════════════════"

_wpa-go:
	@echo "[wpa] starting wpa_supplicant as P2P-GO..."
	@wpa_supplicant \
	    -i $(PHYS_IFACE) \
	    -c config/go.conf \
	    -D nl80211 \
	    -P $(WPA_PID) \
	    -B
	@sleep 2

_p2p-iface:
	@echo "[iface] waiting for p2p interface..."
	@bash scripts/find_p2p_iface.sh > $(P2P_IFACE_F)
	@echo "  found: $$(cat $(P2P_IFACE_F))"

_ip-go:
	@echo "[ip] assigning $(GO_CIDR) to $$(cat $(P2P_IFACE_F))..."
	@ip addr add $(GO_CIDR) dev $$(cat $(P2P_IFACE_F)) 2>/dev/null || true
	@ip link set $$(cat $(P2P_IFACE_F)) up

_dhcp:
	@echo "[dhcp] starting dnsmasq..."
	@sed "s/PLACEHOLDER/$$(cat $(P2P_IFACE_F))/" config/dnsmasq.conf \
	    > /tmp/dnsmasq_p2p.conf
	@dnsmasq \
	    -C /tmp/dnsmasq_p2p.conf \
	    --pid-file=$(DNSMASQ_PID)
	@echo "  dnsmasq running (pid $$(cat $(DNSMASQ_PID)))"

# ─────────────────────────────────────────────
#  Client setup (run on all other nodes)
# ─────────────────────────────────────────────
client: deps _kill-wpa _wpa-client _wait-ip-client
	@echo ""
	@echo "══════════════════════════════════════"
	@echo "  P2P Client ready"
	@echo "  Interface : $(PHYS_IFACE)"
	@echo "  IP        : $$(cat /tmp/p2p_client_ip)"
	@echo "══════════════════════════════════════"

_wpa-client:
	@echo "[wpa] starting wpa_supplicant as P2P-Client..."
	@wpa_supplicant \
	    -i $(PHYS_IFACE) \
	    -c config/client.conf \
	    -D nl80211 \
	    -P $(WPA_PID) \
	    -B
	@sleep 2
	@echo "[dhcp] requesting IP from GO..."
	@dhclient $(PHYS_IFACE) &

_wait-ip-client:
	@echo "[ip] waiting for DHCP assignment..."
	@bash scripts/wait_for_ip.sh $(PHYS_IFACE) > /tmp/p2p_client_ip
	@echo "  got IP: $$(cat /tmp/p2p_client_ip)"

# ─────────────────────────────────────────────
#  Launch application
# ─────────────────────────────────────────────
app:
	@if [ -f $(P2P_IFACE_F) ]; then \
	    IP=$(GO_IP); \
	else \
	    IP=$$(cat /tmp/p2p_client_ip 2>/dev/null || \
	          ip -4 addr show $(PHYS_IFACE) | grep -oP '(?<=inet )[\d.]+'); \
	fi; \
	echo "[app] launching with IP=$$IP bcast=$(BCAST)"; \
	$(APP) --bind $$IP --broadcast $(BCAST)

# ─────────────────────────────────────────────
#  Convenience: setup + launch in one shot
# ─────────────────────────────────────────────
run-go: go app

run-client: client app

# ─────────────────────────────────────────────
#  Status
# ─────────────────────────────────────────────
status:
	@echo "── wpa_supplicant ──────────────────────"
	@cat $(WPA_PID) 2>/dev/null | xargs ps -p 2>/dev/null || echo "  not running"
	@echo "── p2p interface ───────────────────────"
	@ip link show $$(cat $(P2P_IFACE_F) 2>/dev/null) 2>/dev/null || \
	    ip link show $(PHYS_IFACE) 2>/dev/null || echo "  not found"
	@echo "── ip address ──────────────────────────"
	@ip -4 addr show $$(cat $(P2P_IFACE_F) 2>/dev/null || echo $(PHYS_IFACE))
	@echo "── dnsmasq ─────────────────────────────"
	@cat $(DNSMASQ_PID) 2>/dev/null | xargs ps -p 2>/dev/null || echo "  not running"
	@echo "── connected peers ─────────────────────"
	@iw dev $$(cat $(P2P_IFACE_F) 2>/dev/null || echo $(PHYS_IFACE)) station dump \
	    2>/dev/null || echo "  none / not applicable"

# ─────────────────────────────────────────────
#  Teardown
# ─────────────────────────────────────────────
_kill-wpa:
	@if [ -f $(WPA_PID) ]; then \
	    echo "[stop] killing existing wpa_supplicant..."; \
	    kill $$(cat $(WPA_PID)) 2>/dev/null || true; \
	    rm -f $(WPA_PID); \
	    sleep 1; \
	fi

stop: _kill-wpa
	@echo "[stop] stopping dnsmasq..."
	@[ -f $(DNSMASQ_PID) ] && kill $$(cat $(DNSMASQ_PID)) 2>/dev/null || true
	@rm -f $(DNSMASQ_PID)
	@echo "[stop] releasing DHCP lease..."
	@dhclient -r $(PHYS_IFACE) 2>/dev/null || true
	@echo "[stop] flushing IPs..."
	@P2P=$$(cat $(P2P_IFACE_F) 2>/dev/null); \
	 [ -n "$$P2P" ] && ip addr flush dev $$P2P 2>/dev/null || true
	@ip addr flush dev $(PHYS_IFACE) 2>/dev/null || true
	@echo "done."

clean: stop
	@rm -f $(P2P_IFACE_F) /tmp/p2p_client_ip /tmp/dnsmasq_p2p.conf
	@rm -f $(WPA_CTRL)/$(PHYS_IFACE)
	@echo "cleaned."
