# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Standing goals. No state of its own: open intentions are derived from the journal.

qt_add_library(cybou-intentions STATIC
    src/Intentions.cpp
)

target_include_directories(cybou-intentions
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-intentions
    PUBLIC Qt6::Core cybou-storage
)
