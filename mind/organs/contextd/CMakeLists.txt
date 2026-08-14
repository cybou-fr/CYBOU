# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-context-service STATIC
    service/ContextService.cpp
    service/ContextService.h
)

target_include_directories(cybou-context-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-context-service
    PUBLIC Qt6::Core Qt6::DBus cybou-context cybou-events cybou-fabric cybou-protocol
)

qt_add_executable(cybou-contextd service/main.cpp)

target_link_libraries(cybou-contextd
    PRIVATE
        Qt6::Core
        Qt6::DBus
        cybou-context-service
        cybou-event-client
        cybou-fabric
        cybou-runtime
)

install(TARGETS cybou-contextd RUNTIME DESTINATION bin)
