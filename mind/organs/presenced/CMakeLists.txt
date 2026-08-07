# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-presence STATIC
    src/Presence.cpp
    include/cybou/presence/Presence.h
)

target_include_directories(cybou-presence
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-presence
    PUBLIC
        Qt6::Core
        cybou-events
        cybou-event-client
        cybou-runtime
        cybou-self
        cybou-storage
        cybou-workspace
)
