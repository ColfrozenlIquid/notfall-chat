#!/usr/bin/env bash
# =============================================================================
# wifi_direct_host.sh  —  Start a WiFi Direct group (you are the Group Owner)
#
# Requirements: wpa_supplicant (with P2P), wpa_cli, dnsmasq, iw, ip
#   Install:  sudo apt install wpasupplicant dnsmasq iw
#
# Usage: sudo bash wifi_direct_host.sh [wifi-interface]
#   e.g. sudo bash wifi_direct_host.sh wlan0
# =============================================================================

set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────────────
WIFI_IF="${1:-wlan0}"           # Physical WiFi interface
P2P_IP="192.168.100.1"         # IP this machine gets on the P2P link
DHCP_RANGE="192.168.100.10,192.168.100.50"
WPA_CONF="/tmp/wpa_p2p_host.conf"
DNSMASQ_CONF="/tmp/dnsmasq_p2p.conf"
WPA_SOCK_DIR="/var/run/wpa_supplicant"

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
    kill "$DNSMASQ_PID" 2>/dev/null || true
    kill "$WPA_PID"     2>/dev/null || true
    rm -f "$WPA_CONF" "$DNSMASQ_CONF"
    info "Done."
}

# ── Preflight ─────────────────────────────────────────────────────────────────
[[ $EUID -eq 0 ]] || die "Run as root (sudo)."
require wpa_supplicant
require wpa_cli
require dnsmasq
require iw
require ip

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
device_name=P2P-Host
device_type=1-0050F204-1
p2p_go_intent=15
p2p_go_ht40=1
country=DE
EOF

# ── Start wpa_supplicant ──────────────────────────────────────────────────────
info "Starting wpa_supplicant on $WIFI_IF…"
mkdir -p "$WPA_SOCK_DIR"
wpa_supplicant -B -i "$WIFI_IF" -c "$WPA_CONF" -D nl80211 \
    -f /tmp/wpa_p2p_host.log
WPA_PID=$(pgrep -f "wpa_supplicant.*$WIFI_IF" | head -1)
trap cleanup EXIT INT TERM
sleep 2

# ── Create P2P group (autonomous GO) ─────────────────────────────────────────
info "Creating P2P group (this machine = Group Owner)…"
wpa_cli -i "$WIFI_IF" p2p_group_add

# Wait for the new p2p-* interface to appear
info "Waiting for P2P interface to appear…"
for i in $(seq 1 15); do
    P2P_IF=$(iw dev | awk '/Interface p2p-/{print $2}' | head -1)
    [[ -n "$P2P_IF" ]] && break
    sleep 1
done
[[ -n "${P2P_IF:-}" ]] || die "P2P interface never appeared. Check /tmp/wpa_p2p_host.log"
info "P2P interface: $P2P_IF"

# ── Assign IP ─────────────────────────────────────────────────────────────────
info "Assigning IP $P2P_IP to $P2P_IF…"
ip addr add "${P2P_IP}/24" dev "$P2P_IF"
ip link set "$P2P_IF" up

# ── Start dnsmasq (DHCP) ──────────────────────────────────────────────────────
info "Starting DHCP server on $P2P_IF…"
cat > "$DNSMASQ_CONF" <<EOF
interface=${P2P_IF}
bind-interfaces
dhcp-range=${DHCP_RANGE},12h
dhcp-option=3,${P2P_IP}
dhcp-option=6,${P2P_IP}
log-dhcp
EOF

dnsmasq --conf-file="$DNSMASQ_CONF" --pid-file=/tmp/dnsmasq_p2p.pid
DNSMASQ_PID=$(cat /tmp/dnsmasq_p2p.pid)

# ── Show connection info for the client ──────────────────────────────────────
info "Getting P2P group info for the client…"
sleep 1
GROUP_INFO=$(wpa_cli -i "$WIFI_IF" p2p_group_info 2>/dev/null || true)
SSID=$(wpa_cli -i "$P2P_IF" status 2>/dev/null | grep '^ssid=' | cut -d= -f2-)
PASSPHRASE=$(wpa_cli -i "$P2P_IF" status 2>/dev/null | grep '^passphrase=' | cut -d= -f2-)
HOST_MAC=$(cat /sys/class/net/"$WIFI_IF"/address)

echo ""
echo "═══════════════════════════════════════════════════════"
echo "  ✅  WiFi Direct group is RUNNING"
echo "═══════════════════════════════════════════════════════"
echo "  Host MAC  : $HOST_MAC"
echo "  P2P iface : $P2P_IF"
echo "  Host IP   : $P2P_IP"
echo "  SSID      : ${SSID:-<check below>}"
echo "  Password  : ${PASSPHRASE:-<check below>}"
echo ""
echo "  Run on the CLIENT machine:"
echo "    sudo bash wifi_direct_client.sh $WIFI_IF $HOST_MAC"
echo "═══════════════════════════════════════════════════════"
echo ""
echo "Press Ctrl+C to stop."

# ── Keep running ──────────────────────────────────────────────────────────────
wait
