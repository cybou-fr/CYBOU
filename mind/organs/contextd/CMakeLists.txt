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
