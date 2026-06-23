#!/usr/bin/env bash
# =============================================================================
# wifi_direct_client.sh  —  Join an existing WiFi Direct group
#
# Requirements: wpa_supplicant (with P2P), wpa_cli, iw, ip, dhclient/udhcpc
#   Install:  sudo apt install wpasupplicant iw isc-dhcp-client
#
# Usage: sudo bash wifi_direct_client.sh [wifi-interface] [host-MAC]
#   e.g. sudo bash wifi_direct_client.sh wlan0 aa:bb:cc:dd:ee:ff
#
# If you don't know the host MAC, omit it — the script will scan and list peers.
# =============================================================================

set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────────────
WIFI_IF="${1:-wlan0}"
HOST_MAC="${2:-}"               # Optional: MAC of the host machine
WPA_CONF="/tmp/wpa_p2p_client.conf"
WPA_SOCK_DIR="/var/run/wpa_supplicant"
SCAN_SECONDS=15                 # How long to scan for peers

# ── Helpers ───────────────────────────────────────────────────────────────────
info()  { echo -e "\e[32m[+]\e[0m $*"; }
warn()  { echo -e "\e[33m[!]\e[0m $*"; }
die()   { echo -e "\e[31m[✗]\e[0m $*" >&2; exit 1; }

require() { command -v "$1" &>/dev/null || die "'$1' not found. Install it first."; }

cleanup() {
    info "Cleaning up…"
    [[ -n "${P2P_IF:-}" ]] && {
        wpa_cli -i "$WIFI_IF" p2p_group_remove "$P2P_IF" 2>/dev/null || true
        ip addr flush dev "$P2P_IF" 2>/dev/null || true
    }
    kill "$WPA_PID" 2>/dev/null || true
    rm -f "$WPA_CONF"
    info "Done."
}

# ── Preflight ─────────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]] || die "Run as root (sudo)."
require wpa_supplicant
require wpa_cli
require iw
require ip

# Prefer dhclient; fall back to udhcpc
if command -v dhclient &>/dev/null; then
    DHCP_CMD="dhclient"
elif command -v udhcpc &>/dev/null; then
    DHCP_CMD="udhcpc -i"
else
    die "No DHCP client found. Install isc-dhcp-client: sudo apt install isc-dhcp-client"
fi

iw dev "$WIFI_IF" info &>/dev/null || die "Interface '$WIFI_IF' not found. Check with: iw dev"

# ── Stop conflicting services ──────────────────────────────────────────────────
info "Stopping NetworkManager / wpa_supplicant on $WIFI_IF…"
systemctl stop NetworkManager 2>/dev/null || true
pkill -f "wpa_supplicant.*$WIFI_IF" 2>/dev/null || true
sleep 1

# ── Write wpa_supplicant config ───────────────────────────────────────────────
info "Writing wpa_supplicant config…"
cat > "$WPA_CONF" <<EOF
ctrl_interface=${WPA_SOCK_DIR}
ctrl_interface_group=0
device_name=P2P-Client
device_type=1-0050F204-1
p2p_go_intent=0
country=DE
EOF

# ── Start wpa_supplicant ──────────────────────────────────────────────────────
info "Starting wpa_supplicant on $WIFI_IF…"
mkdir -p "$WPA_SOCK_DIR"
wpa_supplicant -B -i "$WIFI_IF" -c "$WPA_CONF" -D nl80211 \
    -f /tmp/wpa_p2p_client.log
WPA_PID=$(pgrep -f "wpa_supplicant.*$WIFI_IF" | head -1)
trap cleanup EXIT INT TERM
sleep 2

# ── P2P Scan ──────────────────────────────────────────────────────────────────
info "Scanning for P2P peers (${SCAN_SECONDS}s)…"
wpa_cli -i "$WIFI_IF" p2p_find

sleep "$SCAN_SECONDS"
wpa_cli -i "$WIFI_IF" p2p_stop_find

# List found peers
PEERS=$(wpa_cli -i "$WIFI_IF" p2p_peers 2>/dev/null || true)
if [[ -z "$PEERS" ]]; then
    die "No P2P peers found. Make sure the host is running wifi_direct_host.sh and is nearby."
fi

echo ""
echo "Found P2P peers:"
echo "─────────────────────────────────"
i=1
declare -a PEER_LIST
while IFS= read -r mac; do
    [[ -z "$mac" ]] && continue
    NAME=$(wpa_cli -i "$WIFI_IF" p2p_peer "$mac" 2>/dev/null | grep '^device_name=' | cut -d= -f2- || echo "unknown")
    echo "  [$i] $mac  ($NAME)"
    PEER_LIST+=("$mac")
    (( i++ ))
done <<< "$PEERS"
echo "─────────────────────────────────"

# ── Select peer ───────────────────────────────────────────────────────────────
if [[ -n "$HOST_MAC" ]]; then
    TARGET_MAC="$HOST_MAC"
    info "Using provided host MAC: $TARGET_MAC"
else
    if [[ ${#PEER_LIST[@]} -eq 1 ]]; then
        TARGET_MAC="${PEER_LIST[0]}"
        info "Auto-selecting the only peer: $TARGET_MAC"
    else
        echo ""
        read -rp "Enter peer number to connect to: " CHOICE
        TARGET_MAC="${PEER_LIST[$((CHOICE - 1))]}"
    fi
fi

# ── Connect via PBC (push-button) ─────────────────────────────────────────────
info "Connecting to $TARGET_MAC via WPS PBC…"
wpa_cli -i "$WIFI_IF" p2p_connect "$TARGET_MAC" pbc go_intent=0

# Wait for the p2p-* client interface to appear
info "Waiting for P2P interface…"
P2P_IF=""
for i in $(seq 1 30); do
    P2P_IF=$(iw dev | awk '/Interface p2p-/{print $2}' | head -1)
    [[ -n "$P2P_IF" ]] && break
    sleep 1
done
[[ -n "${P2P_IF:-}" ]] || die "P2P interface never appeared. Check /tmp/wpa_p2p_client.log"
info "P2P interface: $P2P_IF"

# Wait for association
info "Waiting for association…"
for i in $(seq 1 20); do
    STATUS=$(wpa_cli -i "$P2P_IF" status 2>/dev/null | grep '^wpa_state=' | cut -d= -f2 || true)
    [[ "$STATUS" == "COMPLETED" ]] && break
    sleep 1
done
[[ "${STATUS:-}" == "COMPLETED" ]] || warn "Association may not be complete yet. Trying DHCP anyway…"

# ── Get IP via DHCP ───────────────────────────────────────────────────────────
info "Requesting IP via DHCP on $P2P_IF…"
$DHCP_CMD "$P2P_IF" || die "DHCP failed. Make sure the host's DHCP server is running."

# ── Done ──────────────────────────────────────────────────────────────────────
MY_IP=$(ip addr show "$P2P_IF" | grep 'inet ' | awk '{print $2}' | cut -d/ -f1)
HOST_IP=$(ip route | grep "$P2P_IF" | awk '/via/{print $3}' | head -1 || echo "192.168.100.1")

echo ""
echo "═══════════════════════════════════════════════════════"
echo "  ✅  Connected to WiFi Direct group!"
echo "═══════════════════════════════════════════════════════"
echo "  P2P iface : $P2P_IF"
echo "  My IP     : ${MY_IP:-<check: ip addr show $P2P_IF>}"
echo "  Host IP   : ${HOST_IP:-192.168.100.1}"
echo ""
echo "  Test the link:"
echo "    ping ${HOST_IP:-192.168.100.1}"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "Press Ctrl+C to disconnect."

wait
