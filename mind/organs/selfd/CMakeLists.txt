# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Self-assessment. Owns no facts of its own: it reads the other organs and records what it saw.

qt_add_library(cybou-self STATIC
    src/SelfModel.cpp
)

target_include_directories(cybou-self
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-self
    PUBLIC Qt6::Core cybou-storage cybou-identity cybou-intentions cybou-predictor
)
