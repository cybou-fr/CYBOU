# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

find_package(Qt6 REQUIRED COMPONENTS Test)

qt_add_executable(test-protocol tst_protocol.cpp)
target_link_libraries(test-protocol PRIVATE cybou-protocol Qt6::Test)

# offscreen: the sandbox has no display, and a Qt test without a platform aborts. Same
# failure that core-dumped plasma-apply-desktoptheme during Phase 3.
add_test(NAME protocol COMMAND test-protocol)
set_tests_properties(protocol PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")
