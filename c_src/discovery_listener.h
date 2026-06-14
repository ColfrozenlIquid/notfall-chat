#pragma once
#include "discovery.h"

void discovery_listener_start();

int discovery_listener_pop(DiscoveredPeer* out);
