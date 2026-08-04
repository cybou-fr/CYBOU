# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# The biography. Append-only is enforced by a hash chain, not by convention.

qt_add_library(cybou-storage STATIC
    src/Journal.cpp
)

target_include_directories(cybou-storage
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-storage
    PUBLIC Qt6::Core Qt6::Sql cybou-protocol
)
