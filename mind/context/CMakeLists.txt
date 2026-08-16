# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-context STATIC
    src/AssociativeProjection.cpp
    src/ContextDelivery.cpp
    src/ConsumerRegistry.cpp
    include/cybou/context/AssociativeProjection.h
    include/cybou/context/ContextDelivery.h
    include/cybou/context/ConsumerRegistry.h
)

target_include_directories(cybou-context
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-context
    PUBLIC Qt6::Core cybou-protocol
)
