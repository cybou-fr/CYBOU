# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

qt_add_library(cybou-workspace STATIC
    src/Workspace.cpp
    include/cybou/workspace/Workspace.h
)

target_include_directories(cybou-workspace
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-workspace
    PUBLIC Qt6::Core cybou-events
)

qt_add_library(cybou-workspace-service STATIC
    service/WorkspaceService.cpp
    service/WorkspaceService.h
)

target_include_directories(cybou-workspace-service
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/service>
)

target_link_libraries(cybou-workspace-service
    PUBLIC
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-workspace
)

qt_add_executable(cybou-workspaced
    service/main.cpp
)

target_link_libraries(cybou-workspaced
    PRIVATE
        Qt6::Core
        cybou-event-client
        cybou-fabric
        cybou-workspace-service
)

install(TARGETS cybou-workspaced
    RUNTIME DESTINATION bin
)
