# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# The global workspace. Orders what organs offered; decides nothing itself.

qt_add_library(cybou-workspace STATIC
    src/Workspace.cpp
    include/cybou/workspace/Workspace.h
)

target_include_directories(cybou-workspace
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-workspace
    PUBLIC Qt6::Core cybou-storage
)
