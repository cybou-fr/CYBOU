# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The QML surface still instantiates Presence in plasmashell, but Presence now talks to the
# process-isolated cybou-eventd over Event1. Remaining organs move out in M4.

qt_add_qml_module(cybou-presence-qml
    URI org.cybou.presence
    VERSION 1.0
    PLUGIN_TARGET cybou-presence-qml
    NO_PLUGIN_OPTIONAL
    SOURCES
        QmlTypes.cpp
        QmlTypes.h
)

target_link_libraries(cybou-presence-qml PRIVATE Qt6::Qml cybou-presence)

if(DEFINED CYBOU_QML_INSTALL_DIR)
    set(_dest "${CYBOU_QML_INSTALL_DIR}/org/cybou/presence")
    install(TARGETS cybou-presence-qml LIBRARY DESTINATION "${_dest}")
    install(FILES
        "${CMAKE_CURRENT_BINARY_DIR}/qmldir"
        "${CMAKE_CURRENT_BINARY_DIR}/cybou-presence-qml.qmltypes"
        DESTINATION "${_dest}"
    )
endif()
