# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-predictor STATIC
    src/Predictor.cpp
)

target_include_directories(cybou-predictor
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-predictor
    PUBLIC Qt6::Core cybou-events
)

qt_add_library(cybou-predictor-service STATIC
    service/PredictorService.cpp
    service/PredictorService.h
)

target_include_directories(cybou-predictor-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-predictor-service
    PUBLIC
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-predictor
)

qt_add_executable(cybou-predictord
    service/main.cpp
)

target_link_libraries(cybou-predictord
    PRIVATE
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-predictor-service
)

install(TARGETS cybou-predictord
    RUNTIME DESTINATION bin
)
