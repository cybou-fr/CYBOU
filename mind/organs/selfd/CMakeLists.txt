# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-self STATIC
    src/SelfModel.cpp
)

target_include_directories(cybou-self
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-self
    PUBLIC
        Qt6::Core
        cybou-events
        cybou-identity
        cybou-intentions
        cybou-predictor
)

qt_add_library(cybou-self-service STATIC
    service/SelfService.cpp
    service/SelfService.h
)

target_include_directories(cybou-self-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-self-service
    PUBLIC
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-self
)

qt_add_executable(cybou-selfd
    service/main.cpp
)

target_link_libraries(cybou-selfd
    PRIVATE
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-self-service
)

install(TARGETS cybou-selfd
    RUNTIME DESTINATION bin
)
