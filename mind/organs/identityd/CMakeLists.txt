# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-identity STATIC
    src/Identity.cpp
)

target_include_directories(cybou-identity
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-identity
    PUBLIC Qt6::Core cybou-events
)

qt_add_library(cybou-identity-service STATIC
    service/IdentityService.cpp
    service/IdentityService.h
)

target_include_directories(cybou-identity-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-identity-service
    PUBLIC
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-identity
        cybou-runtime
)

qt_add_executable(cybou-identityd
    service/main.cpp
)

target_link_libraries(cybou-identityd
    PRIVATE
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-identity-service
        cybou-runtime
)

install(TARGETS cybou-identityd
    RUNTIME DESTINATION bin
)
