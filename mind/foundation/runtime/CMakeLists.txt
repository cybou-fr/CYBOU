# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

qt_add_library(cybou-runtime STATIC
    src/StatePaths.cpp
    include/cybou/runtime/StatePaths.h
)

target_include_directories(cybou-runtime
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-runtime
    PUBLIC Qt6::Core
)
