# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Cybou Mind. See docs/14-mind-architecture.md for what this becomes, and
# docs/adr/0008-cognition-development-model.md for what may depend on it.

cmake_minimum_required(VERSION 3.28)

project(cybou-mind VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
# clangd reads this; without it the editor cannot resolve Qt headers.
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

find_package(Qt6 REQUIRED COMPONENTS Core Sql)

qt_standard_project_setup()

add_subdirectory(protocol)
add_subdirectory(foundation/storage)
add_subdirectory(organs/identityd)
add_subdirectory(organs/intentiond)
add_subdirectory(organs/predictord)
add_subdirectory(organs/selfd)
add_subdirectory(organs/workspaced)
add_subdirectory(organs/presenced)

include(CTest)
if(BUILD_TESTING)
    add_subdirectory(tests)
endif()
