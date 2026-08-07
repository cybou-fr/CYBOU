# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

qt_add_library(cybou-events STATIC
    src/EnvelopeCodec.cpp
    include/cybou/events/EventBus.h
    include/cybou/events/EventStore.h
    include/cybou/events/EnvelopeCodec.h
)

target_include_directories(cybou-events
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-events
    PUBLIC Qt6::Core cybou-protocol
)
