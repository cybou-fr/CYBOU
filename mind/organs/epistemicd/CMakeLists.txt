# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

qt_add_library(cybou-epistemic-service STATIC
    service/EpistemicService.cpp
    service/EpistemicService.h
)

target_include_directories(cybou-epistemic-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-epistemic-service
    PUBLIC Qt6::Core cybou-epistemic cybou-events cybou-fabric cybou-protocol
)

qt_add_executable(cybou-epistemicd service/main.cpp)

target_link_libraries(cybou-epistemicd
    PRIVATE
        Qt6::Core
        Qt6::DBus
        cybou-epistemic-service
        cybou-event-client
        cybou-fabric
        cybou-runtime
)

install(TARGETS cybou-epistemicd RUNTIME DESTINATION bin)
