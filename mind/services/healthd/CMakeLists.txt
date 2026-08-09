# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

qt_add_library(cybou-health-service STATIC
    HealthPolicy.cpp
    HealthPolicy.h
    HealthService.cpp
    HealthService.h
)

target_include_directories(cybou-health-service PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(
    cybou-health-service
    PUBLIC Qt6::Core cybou-protocol cybou-fabric cybou-runtime
)

qt_add_executable(cybou-healthd main.cpp)
target_link_libraries(
    cybou-healthd PRIVATE Qt6::Core cybou-health-service cybou-fabric cybou-runtime
)
install(TARGETS cybou-healthd RUNTIME DESTINATION bin)
