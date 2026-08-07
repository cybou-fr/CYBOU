# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-intentions STATIC
    src/Intentions.cpp
)

target_include_directories(cybou-intentions
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-intentions
    PUBLIC Qt6::Core cybou-events
)
