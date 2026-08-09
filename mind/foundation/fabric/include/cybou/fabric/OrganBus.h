// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

namespace cybou {

struct BusEndpoint {
    const char *service;
    const char *objectPath;
    const char *interfaceName;
    const char *systemdUnit;
};

inline constexpr int kFabricIpcVersion = 1;

inline constexpr BusEndpoint kIdentityEndpoint{
    "org.cybou.Mind.Identity1",
    "/org/cybou/Mind/Identity1",
    "org.cybou.Mind.Identity1",
    "cybou-identityd.service",
};

inline constexpr BusEndpoint kIntentionEndpoint{
    "org.cybou.Mind.Intention1",
    "/org/cybou/Mind/Intention1",
    "org.cybou.Mind.Intention1",
    "cybou-intentiond.service",
};

inline constexpr BusEndpoint kPredictorEndpoint{
    "org.cybou.Mind.Predictor1",
    "/org/cybou/Mind/Predictor1",
    "org.cybou.Mind.Predictor1",
    "cybou-predictord.service",
};

inline constexpr BusEndpoint kSelfEndpoint{
    "org.cybou.Mind.Self1",
    "/org/cybou/Mind/Self1",
    "org.cybou.Mind.Self1",
    "cybou-selfd.service",
};

inline constexpr BusEndpoint kWorkspaceEndpoint{
    "org.cybou.Mind.Workspace1",
    "/org/cybou/Mind/Workspace1",
    "org.cybou.Mind.Workspace1",
    "cybou-workspaced.service",
};

inline constexpr BusEndpoint kPresenceEndpoint{
    "org.cybou.Mind.Presence1",
    "/org/cybou/Mind/Presence1",
    "org.cybou.Mind.Presence1",
    "cybou-presenced.service",
};

inline constexpr BusEndpoint kLifecycleEndpoint{
    "org.cybou.Mind.Lifecycle1", "/org/cybou/Mind/Lifecycle1",
    "org.cybou.Mind.Lifecycle1", "cybou-lifecycled.service",
};

} // namespace cybou
