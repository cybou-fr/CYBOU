# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

# QML-side proxy only. This target must not link organ domain libraries.
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
        cybou-fabric
        cybou-runtime
)

qt_add_library(cybou-presence-service STATIC
    service/PresenceService.cpp
    service/PresenceService.h
)

target_include_directories(cybou-presence-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-presence-service
    PUBLIC
        Qt6::Core
        cybou-event-client
        cybou-events
        cybou-fabric
)

qt_add_executable(cybou-presenced
    service/main.cpp
)

target_link_libraries(cybou-presenced
    PRIVATE
        Qt6::Core
        cybou-fabric
        cybou-presence-service
)

install(TARGETS cybou-presenced
    RUNTIME DESTINATION bin
)
