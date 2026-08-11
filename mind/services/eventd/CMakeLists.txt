# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

qt_add_library(cybou-eventd-core STATIC
    src/EventService.cpp
    include/cybou/eventd/EventService.h
)

target_include_directories(cybou-eventd-core
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

# Qt6::DBus is PUBLIC because EventService derives from QDBusContext: it has to see the calling
# process to check that a contribution's claimed origin is one that caller may use.
target_link_libraries(cybou-eventd-core
    PUBLIC Qt6::Core Qt6::DBus cybou-events cybou-storage
)

qt_add_executable(cybou-eventd
    src/main.cpp
)

target_link_libraries(cybou-eventd
    PRIVATE
        Qt6::Core
        Qt6::DBus
        cybou-eventd-core
        cybou-fabric
        cybou-runtime
)

install(TARGETS cybou-eventd
    RUNTIME DESTINATION bin
)
