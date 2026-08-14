# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

find_package(PkgConfig REQUIRED)
pkg_check_modules(SODIUM REQUIRED IMPORTED_TARGET libsodium)

qt_add_library(cybou-crypto STATIC
    src/SealedPayload.cpp
    include/cybou/crypto/SealedPayload.h
)

target_include_directories(cybou-crypto
    PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
)

target_link_libraries(cybou-crypto
    PUBLIC Qt6::Core
    PRIVATE PkgConfig::SODIUM
)
