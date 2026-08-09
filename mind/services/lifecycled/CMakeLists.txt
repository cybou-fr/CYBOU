# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
qt_add_library(cybou-lifecycle-service STATIC LifecycleService.cpp LifecycleService.h)
target_include_directories(cybou-lifecycle-service PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(cybou-lifecycle-service PUBLIC Qt6::Core cybou-protocol cybou-fabric cybou-runtime cybou-event-client)
qt_add_executable(cybou-lifecycled main.cpp)
target_link_libraries(cybou-lifecycled PRIVATE Qt6::Core cybou-lifecycle-service cybou-fabric cybou-runtime)
install(TARGETS cybou-lifecycled RUNTIME DESTINATION bin)
