# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The QML type the panel instantiates. In-process on purpose for the current prototype:
# no daemon, no bus, no listener. The Plasma package owns all visual QML files; this
# module exports only the C++ Presence type under org.cybou.presence.

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

# Installed only when a destination is given, so a plain developer build stays in the build
# tree and never writes outside it.
if(DEFINED CYBOU_QML_INSTALL_DIR)
    set(_dest "${CYBOU_QML_INSTALL_DIR}/org/cybou/presence")
    install(TARGETS cybou-presence-qml LIBRARY DESTINATION "${_dest}")
    install(FILES
        "${CMAKE_CURRENT_BINARY_DIR}/qmldir"
        "${CMAKE_CURRENT_BINARY_DIR}/cybou-presence-qml.qmltypes"
        DESTINATION "${_dest}"
    )
endif()
