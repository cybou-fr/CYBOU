# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

cmake_minimum_required(VERSION 3.28)

project(cybou-mind VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

find_package(Qt6 REQUIRED COMPONENTS Core DBus Sql Qml)

qt_standard_project_setup()

add_subdirectory(protocol)
add_subdirectory(foundation/events)
add_subdirectory(foundation/runtime)
add_subdirectory(foundation/storage)
add_subdirectory(foundation/ipc)
add_subdirectory(foundation/fabric)
add_subdirectory(perception)
add_subdirectory(epistemic)
add_subdirectory(services/eventd)
add_subdirectory(services/healthd)
add_subdirectory(services/lifecycled)

add_subdirectory(organs/identityd)
add_subdirectory(organs/intentiond)
add_subdirectory(organs/predictord)
add_subdirectory(organs/selfd)
add_subdirectory(organs/workspaced)
add_subdirectory(organs/perceptiond)
add_subdirectory(organs/presenced)
add_subdirectory(shell/qmlplugin)

include(CTest)
if(BUILD_TESTING)
    add_subdirectory(tests)
endif()
