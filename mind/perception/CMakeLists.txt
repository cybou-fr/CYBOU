# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The first perception source. A library rather than a process for now: the acquisition logic and
# its typed failure result are the substantive part, and the process that carries it onto the bus is
# added separately so the two can be reviewed apart.

qt_add_library(cybou-perception STATIC
    src/SystemGenerationSource.cpp
    include/cybou/perception/SystemGenerationSource.h
)

target_include_directories(cybou-perception
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-perception
    PUBLIC Qt6::Core cybou-protocol
)
