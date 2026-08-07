# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-intentions STATIC
    src/Intentions.cpp
)

target_include_directories(cybou-intentions
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-intentions
    PUBLIC Qt6::Core cybou-events
)

qt_add_library(cybou-intention-service STATIC
    service/IntentionService.cpp
    service/IntentionService.h
)

target_include_directories(cybou-intention-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-intention-service
    PUBLIC
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-intentions
)

qt_add_executable(cybou-intentiond
    service/main.cpp
)

target_link_libraries(cybou-intentiond
    PRIVATE
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-intention-service
)

install(TARGETS cybou-intentiond
    RUNTIME DESTINATION bin
)
