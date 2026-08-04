# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

find_package(Qt6 REQUIRED COMPONENTS Test)

qt_add_executable(test-protocol tst_protocol.cpp)
target_link_libraries(test-protocol PRIVATE cybou-protocol Qt6::Test)

# offscreen: the sandbox has no display, and a Qt test without a platform aborts. Same
# failure that core-dumped plasma-apply-desktoptheme during Phase 3.
add_test(NAME protocol COMMAND test-protocol)
set_tests_properties(protocol PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-journal tst_journal.cpp)
target_link_libraries(test-journal PRIVATE cybou-storage Qt6::Test)
add_test(NAME journal COMMAND test-journal)
set_tests_properties(journal PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-identity tst_identity.cpp)
target_link_libraries(test-identity PRIVATE cybou-identity Qt6::Test)
add_test(NAME identity COMMAND test-identity)
set_tests_properties(identity PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")
