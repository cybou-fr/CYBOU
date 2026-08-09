# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

qt_add_library(cybou-fabric STATIC
    src/FabricCodec.cpp
    src/RpcClient.cpp
    src/RpcResilience.cpp
    src/ServiceHost.cpp
    src/OrganClients.cpp
    include/cybou/fabric/FabricCodec.h
    include/cybou/fabric/OrganBus.h
    include/cybou/fabric/OrganClients.h
    include/cybou/fabric/RpcClient.h
    include/cybou/fabric/RpcResilience.h
    include/cybou/fabric/ServiceHost.h
)

target_include_directories(cybou-fabric
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-fabric
    PUBLIC Qt6::Core Qt6::DBus
)
