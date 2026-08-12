# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The epistemic projection as a library. It derives and decides nothing else: no Journal, no
# transport, no clock of its own. The owning process comes separately, so the reasoning can be
# reviewed and tested apart from the plumbing that will carry it.

qt_add_library(cybou-epistemic STATIC
    src/EpistemicProjection.cpp
    include/cybou/epistemic/EpistemicProjection.h
)

target_include_directories(cybou-epistemic
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-epistemic
    PUBLIC Qt6::Core cybou-protocol
)
