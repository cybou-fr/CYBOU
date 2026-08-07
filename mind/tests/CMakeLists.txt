# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

find_package(Qt6 REQUIRED COMPONENTS Test)
find_program(DBUS_RUN_SESSION_EXECUTABLE dbus-run-session REQUIRED)

qt_add_executable(test-protocol tst_protocol.cpp)
target_link_libraries(test-protocol PRIVATE cybou-protocol Qt6::Test)
add_test(NAME protocol COMMAND test-protocol)
set_tests_properties(protocol PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-journal tst_journal.cpp)
target_link_libraries(test-journal PRIVATE cybou-storage Qt6::Test)
add_test(NAME journal COMMAND test-journal)
set_tests_properties(journal PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-identity tst_identity.cpp)
target_link_libraries(test-identity PRIVATE cybou-identity cybou-storage Qt6::Test)
add_test(NAME identity COMMAND test-identity)
set_tests_properties(identity PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-intentions tst_intentions.cpp)
target_link_libraries(test-intentions PRIVATE cybou-intentions cybou-storage Qt6::Test)
add_test(NAME intentions COMMAND test-intentions)
set_tests_properties(intentions PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-predictor tst_predictor.cpp)
target_link_libraries(test-predictor PRIVATE cybou-predictor cybou-storage Qt6::Test)
add_test(NAME predictor COMMAND test-predictor)
set_tests_properties(predictor PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-selfmodel tst_selfmodel.cpp)
target_link_libraries(
    test-selfmodel
    PRIVATE cybou-self cybou-storage Qt6::Sql Qt6::Test
)
add_test(NAME selfmodel COMMAND test-selfmodel)
set_tests_properties(selfmodel PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-workspace tst_workspace.cpp)
target_link_libraries(
    test-workspace
    PRIVATE cybou-workspace cybou-storage Qt6::Test
)
add_test(NAME workspace COMMAND test-workspace)
set_tests_properties(workspace PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-presence-proxy tst_presence.cpp)
target_link_libraries(
    test-presence-proxy
    PRIVATE cybou-presence Qt6::Test
)
add_test(NAME presence-proxy COMMAND test-presence-proxy)
set_tests_properties(
    presence-proxy
    PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-m1-runtime tst_m1_runtime.cpp)
target_link_libraries(
    test-m1-runtime
    PRIVATE
        cybou-runtime
        cybou-workspace
        cybou-storage
        Qt6::Test
)
add_test(NAME m1-runtime COMMAND test-m1-runtime)
set_tests_properties(m1-runtime PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-fabric-codec tst_fabric_codec.cpp)
target_link_libraries(
    test-fabric-codec
    PRIVATE cybou-fabric Qt6::Test
)
add_test(NAME fabric-codec COMMAND test-fabric-codec)
set_tests_properties(fabric-codec PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-eventd-integration tst_eventd_integration.cpp)
target_link_libraries(
    test-eventd-integration
    PRIVATE
        cybou-event-client
        cybou-events
        Qt6::Test
)
add_test(
    NAME eventd-integration
    COMMAND
        ${DBUS_RUN_SESSION_EXECUTABLE}
        --
        $<TARGET_FILE:test-eventd-integration>
        $<TARGET_FILE:cybou-eventd>
)
set_tests_properties(
    eventd-integration
    PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-m4-process-integration tst_m4_process_integration.cpp)
target_link_libraries(
    test-m4-process-integration
    PRIVATE
        cybou-event-client
        cybou-fabric
        cybou-presence
        Qt6::Test
)
add_test(
    NAME m4-process-integration
    COMMAND
        ${DBUS_RUN_SESSION_EXECUTABLE}
        --
        $<TARGET_FILE:test-m4-process-integration>
        $<TARGET_FILE:cybou-eventd>
        $<TARGET_FILE:cybou-identityd>
        $<TARGET_FILE:cybou-intentiond>
        $<TARGET_FILE:cybou-predictord>
        $<TARGET_FILE:cybou-selfd>
        $<TARGET_FILE:cybou-workspaced>
        $<TARGET_FILE:cybou-presenced>
)
set_tests_properties(
    m4-process-integration
    PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)
