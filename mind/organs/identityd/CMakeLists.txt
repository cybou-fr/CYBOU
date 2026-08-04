# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Continuity of the subject. Holds no memories and makes no decisions.

qt_add_library(cybou-identity STATIC
    src/Identity.cpp
)

target_include_directories(cybou-identity
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-identity
    PUBLIC Qt6::Core cybou-storage
)
