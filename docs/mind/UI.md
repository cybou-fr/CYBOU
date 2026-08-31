<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Living Canvas Spatial UI

Living Canvas is the spatial desktop interface for CYBOU. It delivers 20+ responsive cards
(Terminal, Files, Cognitive Graph, System Monitor, Notes, Calendar) over a GPU-accelerated
Leptos/WebAssembly runtime on Debian 13.

## Principles

1. **Projection, not owner**: The UI is a pure projection of the Mind and Body daemons. It never owns cognitive state, journal writes, or security policy.
2. **Infinite Canvas**: Cards, decks, and clusters live on an unbounded 2D plane with real-time snap guides and multi-mode layout arrangement.
3. **Sub-millisecond responsiveness**: Client interactions run entirely in client-side WebAssembly with zero blocking D-Bus calls.
4. **Resilient streams**: State updates flow continuously over Server-Sent Events (SSE) and WebSocket channels from `cybou-web-gateway`.
