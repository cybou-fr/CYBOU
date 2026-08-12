# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

qt_add_library(cybou-perception-service STATIC
    service/PerceptionService.cpp
    service/PerceptionService.h
)

target_include_directories(cybou-perception-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-perception-service
    PUBLIC Qt6::Core cybou-events cybou-perception cybou-protocol
)

qt_add_executable(cybou-perceptiond service/main.cpp)

target_link_libraries(cybou-perceptiond
    PRIVATE
        Qt6::Core
        Qt6::DBus
        cybou-perception-service
        cybou-event-client
        cybou-fabric
        cybou-runtime
)

install(TARGETS cybou-perceptiond RUNTIME DESTINATION bin)
