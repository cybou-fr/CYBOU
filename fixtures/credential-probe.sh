#!/bin/sh
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Print whatever a service manager handed this service as the credential named "lease".
#
# Used by the credential gate to find out what an unprivileged service actually receives when the
# path it was loaded from is a symlink to a root-only file. It prints rather than judges: the gate
# decides what the answer means, and a probe that decided for itself could only ever confirm what
# somebody already believed.

echo "DIR=${CREDENTIALS_DIRECTORY:-none}"
if [ -n "${CREDENTIALS_DIRECTORY:-}" ] && [ -e "${CREDENTIALS_DIRECTORY}/lease" ]; then
    echo "CONTENT-BEGIN"
    cat "${CREDENTIALS_DIRECTORY}/lease" 2>&1
    echo "CONTENT-END"
else
    echo "NO-CREDENTIAL"
fi
