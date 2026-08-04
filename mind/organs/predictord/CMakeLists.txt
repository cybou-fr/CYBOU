# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Forecasts joined to outcomes. No state of its own: accuracy is derived from the journal.

qt_add_library(cybou-predictor STATIC
    src/Predictor.cpp
)

target_include_directories(cybou-predictor
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-predictor
    PUBLIC Qt6::Core cybou-storage
)
