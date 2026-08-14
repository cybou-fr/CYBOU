# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

find_package(Qt6 REQUIRED COMPONENTS Test)

find_program(DBUS_RUN_SESSION_EXECUTABLE dbus-run-session REQUIRED)
find_program(DBUS_DAEMON_EXECUTABLE dbus-daemon REQUIRED)

# In a Nix build sandbox, dbus-run-session's default `--session` makes dbus-daemon look for
# /etc/dbus-1/session.conf, but the sandbox intentionally does not populate host /etc.
# Resolve the immutable session.conf next to the actual dbus-daemon and pass it explicitly.
get_filename_component(_cybou_dbus_bin_dir "${DBUS_DAEMON_EXECUTABLE}" DIRECTORY)
get_filename_component(_cybou_dbus_prefix "${_cybou_dbus_bin_dir}" DIRECTORY)
set(
    CYBOU_DBUS_SESSION_CONFIG
    "${_cybou_dbus_prefix}/share/dbus-1/session.conf"
)

if(NOT EXISTS "${CYBOU_DBUS_SESSION_CONFIG}")
    message(
        FATAL_ERROR
        "D-Bus session configuration not found at ${CYBOU_DBUS_SESSION_CONFIG}"
    )
endif()

set(CYBOU_DBUS_TEST_RUNNER
    "${DBUS_RUN_SESSION_EXECUTABLE}"
    "--dbus-daemon=${DBUS_DAEMON_EXECUTABLE}"
    "--config-file=${CYBOU_DBUS_SESSION_CONFIG}"
    "--"
)

qt_add_executable(test-protocol tst_protocol.cpp)
target_link_libraries(test-protocol PRIVATE cybou-protocol Qt6::Test)
add_test(NAME protocol COMMAND test-protocol)
set_tests_properties(protocol PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-health-protocol tst_health_protocol.cpp)
target_link_libraries(test-health-protocol PRIVATE cybou-protocol Qt6::Test)
add_test(NAME health-protocol COMMAND test-health-protocol)
set_tests_properties(health-protocol PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-homeostasis-protocol tst_homeostasis_protocol.cpp)
target_link_libraries(test-homeostasis-protocol PRIVATE cybou-protocol Qt6::Test)
add_test(NAME homeostasis-protocol COMMAND test-homeostasis-protocol)
set_tests_properties(homeostasis-protocol PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-health-service tst_health_service.cpp)
target_link_libraries(
    test-health-service PRIVATE cybou-health-service cybou-protocol Qt6::Test
)
add_test(NAME health-service COMMAND test-health-service)
set_tests_properties(health-service PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-healthd-integration tst_healthd_integration.cpp)
target_link_libraries(
    test-healthd-integration PRIVATE cybou-protocol cybou-fabric Qt6::DBus Qt6::Test
)
add_test(
    NAME healthd-integration
    COMMAND
        ${CYBOU_DBUS_TEST_RUNNER}
        ${CMAKE_COMMAND} -E env
        CYBOU_HEALTHD_PATH=$<TARGET_FILE:cybou-healthd>
        CYBOU_EVENTD_PATH=$<TARGET_FILE:cybou-eventd>
        CYBOU_LIFECYCLED_PATH=$<TARGET_FILE:cybou-lifecycled>
        CYBOU_IDENTITYD_PATH=$<TARGET_FILE:cybou-identityd>
        CYBOU_INTENTIOND_PATH=$<TARGET_FILE:cybou-intentiond>
        CYBOU_PREDICTORD_PATH=$<TARGET_FILE:cybou-predictord>
        CYBOU_SELFD_PATH=$<TARGET_FILE:cybou-selfd>
        CYBOU_WORKSPACED_PATH=$<TARGET_FILE:cybou-workspaced>
        CYBOU_PRESENCED_PATH=$<TARGET_FILE:cybou-presenced>
        CYBOU_PERCEPTIOND_PATH=$<TARGET_FILE:cybou-perceptiond>
        CYBOU_EPISTEMICD_PATH=$<TARGET_FILE:cybou-epistemicd>
        $<TARGET_FILE:test-healthd-integration>
)
set_tests_properties(
    healthd-integration PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-lifecycle tst_lifecycle.cpp)
target_link_libraries(test-lifecycle PRIVATE cybou-lifecycle-service cybou-fabric Qt6::Test)
add_test(NAME lifecycle COMMAND test-lifecycle)
set_tests_properties(lifecycle PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-lifecycle-scheduling-policy tst_lifecycle_scheduling_policy.cpp)
target_link_libraries(
    test-lifecycle-scheduling-policy PRIVATE cybou-lifecycle-service cybou-protocol Qt6::Test
)
add_test(NAME lifecycle-scheduling-policy COMMAND test-lifecycle-scheduling-policy)
set_tests_properties(
    lifecycle-scheduling-policy PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-lifecycled-integration tst_lifecycled_integration.cpp)
target_link_libraries(
    test-lifecycled-integration
    PRIVATE cybou-events cybou-fabric Qt6::DBus Qt6::Test
)
add_test(
    NAME lifecycled-integration
    COMMAND
        ${CYBOU_DBUS_TEST_RUNNER}
        ${CMAKE_COMMAND} -E env
        CYBOU_LIFECYCLED_PATH=$<TARGET_FILE:cybou-lifecycled>
        CYBOU_EVENTD_PATH=$<TARGET_FILE:cybou-eventd>
        CYBOU_PREDICTORD_PATH=$<TARGET_FILE:cybou-predictord>
        CYBOU_WORKSPACED_PATH=$<TARGET_FILE:cybou-workspaced>
        $<TARGET_FILE:test-lifecycled-integration>
)
set_tests_properties(
    lifecycled-integration
    PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-journal tst_journal.cpp)
target_link_libraries(test-journal PRIVATE cybou-storage cybou-crypto Qt6::Test)
add_test(NAME journal COMMAND test-journal)
set_tests_properties(journal PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

# Scale baseline. Defaults to 10k so it belongs in the ordinary run; raise
# CYBOU_SCALE_CONTRIBUTIONS to 100000 or 1000000 for the separate scale gate and the manual
# release benchmark. Its measurements are printed, so run ctest with --verbose to read them.
qt_add_executable(test-associative-projection tst_associative_projection.cpp)
target_link_libraries(test-associative-projection PRIVATE cybou-context Qt6::Test)
add_test(NAME associative-projection COMMAND test-associative-projection)
set_tests_properties(
    associative-projection PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-sealed-payload tst_sealed_payload.cpp)
target_link_libraries(test-sealed-payload PRIVATE cybou-crypto Qt6::Test)
add_test(NAME sealed-payload COMMAND test-sealed-payload)
set_tests_properties(
    sealed-payload PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-journal-scale tst_journal_scale.cpp)
target_link_libraries(
    test-journal-scale PRIVATE cybou-storage cybou-predictor cybou-intentions Qt6::Test
)
add_test(NAME journal-scale COMMAND test-journal-scale)
set_tests_properties(journal-scale PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-observation tst_observation.cpp)
target_link_libraries(test-observation PRIVATE cybou-protocol Qt6::Test)
add_test(NAME observation COMMAND test-observation)
set_tests_properties(observation PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-system-generation-source tst_system_generation_source.cpp)
target_link_libraries(
    test-system-generation-source PRIVATE cybou-perception cybou-protocol Qt6::Test
)
add_test(NAME system-generation-source COMMAND test-system-generation-source)
set_tests_properties(
    system-generation-source PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-perception-service tst_perception_service.cpp)
target_link_libraries(
    test-perception-service PRIVATE cybou-perception-service cybou-storage Qt6::Test
)
add_test(NAME perception-service COMMAND test-perception-service)
set_tests_properties(
    perception-service PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-perceptiond-integration tst_perceptiond_integration.cpp)
target_link_libraries(
    test-perceptiond-integration
    PRIVATE cybou-event-client cybou-fabric cybou-protocol Qt6::DBus Qt6::Test
)
add_test(
    NAME perceptiond-integration
    COMMAND
        ${CYBOU_DBUS_TEST_RUNNER}
        ${CMAKE_COMMAND} -E env
        CYBOU_EVENTD_PATH=$<TARGET_FILE:cybou-eventd>
        CYBOU_PERCEPTIOND_PATH=$<TARGET_FILE:cybou-perceptiond>
        $<TARGET_FILE:test-perceptiond-integration>
)
set_tests_properties(
    perceptiond-integration PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-epistemicd-integration tst_epistemicd_integration.cpp)
target_link_libraries(
    test-epistemicd-integration
    PRIVATE cybou-event-client cybou-fabric cybou-protocol Qt6::DBus Qt6::Test
)
add_test(
    NAME epistemicd-integration
    COMMAND
        ${CYBOU_DBUS_TEST_RUNNER}
        ${CMAKE_COMMAND} -E env
        CYBOU_EVENTD_PATH=$<TARGET_FILE:cybou-eventd>
        CYBOU_PERCEPTIOND_PATH=$<TARGET_FILE:cybou-perceptiond>
        CYBOU_EPISTEMICD_PATH=$<TARGET_FILE:cybou-epistemicd>
        $<TARGET_FILE:test-epistemicd-integration>
)
set_tests_properties(
    epistemicd-integration PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-epistemic-projection tst_epistemic_projection.cpp)
target_link_libraries(
    test-epistemic-projection PRIVATE cybou-epistemic cybou-protocol Qt6::Test
)
add_test(NAME epistemic-projection COMMAND test-epistemic-projection)
set_tests_properties(
    epistemic-projection PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-epistemic-service tst_epistemic_service.cpp)
target_link_libraries(
    test-epistemic-service PRIVATE cybou-epistemic-service cybou-fabric cybou-storage Qt6::Test
)
add_test(NAME epistemic-service COMMAND test-epistemic-service)
set_tests_properties(
    epistemic-service PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)

qt_add_executable(test-identity tst_identity.cpp)
target_link_libraries(test-identity PRIVATE cybou-identity cybou-storage Qt6::Test)
add_test(NAME identity COMMAND test-identity)
set_tests_properties(identity PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-intentions tst_intentions.cpp)
target_link_libraries(test-intentions PRIVATE cybou-intentions cybou-storage Qt6::Test)
add_test(NAME intentions COMMAND test-intentions)
set_tests_properties(intentions PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

qt_add_executable(test-predictor tst_predictor.cpp)
target_link_libraries(
    test-predictor PRIVATE cybou-predictor-service cybou-fabric cybou-storage Qt6::Test
)
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
    PRIVATE cybou-workspace-service cybou-fabric cybou-storage Qt6::Test
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

qt_add_executable(test-rpc-resilience tst_rpc_resilience.cpp)
target_link_libraries(test-rpc-resilience PRIVATE cybou-fabric Qt6::DBus Qt6::Test)
add_test(
    NAME rpc-resilience
    COMMAND ${CYBOU_DBUS_TEST_RUNNER} $<TARGET_FILE:test-rpc-resilience>
)
set_tests_properties(rpc-resilience PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")

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
        ${CYBOU_DBUS_TEST_RUNNER}
        ${CMAKE_COMMAND} -E env
        CYBOU_EVENTD_PATH=$<TARGET_FILE:cybou-eventd>
        $<TARGET_FILE:test-eventd-integration>
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
        cybou-eventd-core
        cybou-fabric
        cybou-presence
        Qt6::Test
)
add_test(
    NAME m4-process-integration
    COMMAND
        ${CYBOU_DBUS_TEST_RUNNER}
        ${CMAKE_COMMAND} -E env
        CYBOU_EVENTD_PATH=$<TARGET_FILE:cybou-eventd>
        CYBOU_IDENTITYD_PATH=$<TARGET_FILE:cybou-identityd>
        CYBOU_INTENTIOND_PATH=$<TARGET_FILE:cybou-intentiond>
        CYBOU_PREDICTORD_PATH=$<TARGET_FILE:cybou-predictord>
        CYBOU_SELFD_PATH=$<TARGET_FILE:cybou-selfd>
        CYBOU_WORKSPACED_PATH=$<TARGET_FILE:cybou-workspaced>
        CYBOU_LIFECYCLED_PATH=$<TARGET_FILE:cybou-lifecycled>
        CYBOU_HEALTHD_PATH=$<TARGET_FILE:cybou-healthd>
        CYBOU_PRESENCED_PATH=$<TARGET_FILE:cybou-presenced>
        CYBOU_PERCEPTIOND_PATH=$<TARGET_FILE:cybou-perceptiond>
        CYBOU_EPISTEMICD_PATH=$<TARGET_FILE:cybou-epistemicd>
        $<TARGET_FILE:test-m4-process-integration>
)
set_tests_properties(
    m4-process-integration
    PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
)
