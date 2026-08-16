# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

qt_add_library(cybou-event-client STATIC
    src/EventClient.cpp
    src/CallerIdentity.cpp
    include/cybou/ipc/EventClient.h
    include/cybou/ipc/CallerIdentity.h
)

target_include_directories(cybou-event-client
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-event-client
    PUBLIC Qt6::Core Qt6::DBus cybou-events
)
