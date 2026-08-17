# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# The shared language. Every organ links this and nothing else to talk to another organ.

qt_add_library(cybou-protocol STATIC
    src/Observation.cpp
    include/cybou/protocol/Observation.h
    src/Sensitivity.cpp
    include/cybou/protocol/Sensitivity.h
    src/CapabilityRegistry.cpp
    include/cybou/protocol/CapabilityRegistry.h
    src/CanonicalEnvelope.cpp
    src/CognitiveEnvelope.cpp
    src/Health.cpp
    src/Homeostasis.cpp
    src/Lifecycle.cpp
)

target_include_directories(cybou-protocol
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-protocol
    PUBLIC
        Qt6::Core
)

# STATIC for now: one process per organ is the design, and a shared library only earns its
# keep once several of them are installed side by side.
