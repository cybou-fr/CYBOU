# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-meaning STATIC
    src/CognitiveAct.cpp
    src/ResponsePlan.cpp
    include/cybou/meaning/CognitiveAct.h
    include/cybou/meaning/ResponsePlan.h
)

target_include_directories(cybou-meaning
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-meaning
    PUBLIC Qt6::Core cybou-protocol
)
