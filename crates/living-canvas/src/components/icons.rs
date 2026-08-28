// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! SVG icon primitives for the Living Canvas interface.

use leptos::prelude::*;

#[component]
pub fn IconPin(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="17" x2="12" y2="22"></line>
            <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a1 1 0 0 0 0-2H8a1 1 0 0 0 0 2h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path>
        </svg>
    }
}

#[component]
pub fn IconMinimize(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 14 10 14 10 20"></polyline>
            <polyline points="20 10 14 10 14 4"></polyline>
            <line x1="14" y1="10" x2="21" y2="3"></line>
            <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
    }
}

#[component]
pub fn IconMaximize(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 3 21 3 21 9"></polyline>
            <polyline points="9 21 3 21 3 15"></polyline>
            <line x1="21" y1="3" x2="14" y2="10"></line>
            <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
    }
}

#[component]
pub fn IconGrid(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="7"></rect>
            <rect x="14" y="3" width="7" height="7"></rect>
            <rect x="14" y="14" width="7" height="7"></rect>
            <rect x="3" y="14" width="7" height="7"></rect>
        </svg>
    }
}

#[component]
pub fn IconRefresh(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"></path>
            <path d="M21 3v5h-5"></path>
            <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"></path>
            <path d="M8 16H3v5"></path>
        </svg>
    }
}

#[component]
pub fn IconClose(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
    }
}

#[component]
pub fn IconTerminal(#[prop(default = 15)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 17 10 11 4 5"></polyline>
            <line x1="12" y1="19" x2="20" y2="19"></line>
        </svg>
    }
}

#[component]
pub fn IconResizeGrip() -> impl IntoView {
    view! {
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round">
            <line x1="8" y1="2" x2="2" y2="8"></line>
            <line x1="8" y1="5" x2="5" y2="8"></line>
            <line x1="8" y1="8" x2="8" y2="8"></line>
        </svg>
    }
}

#[component]
pub fn IconUndo(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 7v6h6"></path>
            <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"></path>
        </svg>
    }
}

#[component]
pub fn IconRedo(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 7v6h-6"></path>
            <path d="M3 17a9 9 0 0 1 9-9 9 9 0 0 1 6 2.3l3 2.7"></path>
        </svg>
    }
}

#[allow(dead_code)]
#[component]
pub fn IconExternalLink(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M15 3h6v6"></path>
            <path d="M10 14 21 3"></path>
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
        </svg>
    }
}

#[allow(dead_code)]
#[component]
pub fn IconLayers(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"></path>
            <path d="m2 12 8.58 3.91a2 2 0 0 0 1.66 0L21 12"></path>
            <path d="m2 17 8.58 3.91a2 2 0 0 0 1.66 0L21 17"></path>
        </svg>
    }
}

#[component]
pub fn IconShield(#[prop(default = 14)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
        </svg>
    }
}

#[component]
pub fn IconFolder(#[prop(default = 14)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"></path>
        </svg>
    }
}

#[component]
pub fn IconFile(#[prop(default = 14)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"></path>
            <path d="M14 2v4a2 2 0 0 0 2 2h4"></path>
        </svg>
    }
}

#[component]
pub fn IconArrowLeft(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m12 19-7-7 7-7"></path>
            <path d="M19 12H5"></path>
        </svg>
    }
}

#[component]
pub fn IconArrowRight(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m12 5 7 7-7 7"></path>
            <path d="M5 12h14"></path>
        </svg>
    }
}

#[component]
pub fn IconHome(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
            <polyline points="9 22 9 12 15 12 15 22"></polyline>
        </svg>
    }
}

#[component]
pub fn IconZoomIn(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            <line x1="11" y1="8" x2="11" y2="14"></line>
            <line x1="8" y1="11" x2="14" y2="11"></line>
        </svg>
    }
}

#[component]
pub fn IconZoomOut(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            <line x1="8" y1="11" x2="14" y2="11"></line>
        </svg>
    }
}

#[component]
pub fn IconCopy(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect width="14" height="14" x="8" y="8" rx="2" ry="2"></rect>
            <path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"></path>
        </svg>
    }
}

#[component]
pub fn IconSearch(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
        </svg>
    }
}

#[component]
pub fn IconTrash(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
        </svg>
    }
}

#[component]
pub fn IconEdit(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
        </svg>
    }
}

#[component]
pub fn IconBot(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect width="18" height="14" x="3" y="6" rx="2"></rect>
            <line x1="12" y1="2" x2="12" y2="6"></line>
            <line x1="8" y1="12" x2="8" y2="12.01"></line>
            <line x1="16" y1="12" x2="16" y2="12.01"></line>
        </svg>
    }
}

#[component]
pub fn IconPlus(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
    }
}

#[component]
pub fn IconFolderPlus(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"></path>
            <line x1="12" y1="10" x2="12" y2="16"></line>
            <line x1="9" y1="13" x2="15" y2="13"></line>
        </svg>
    }
}

#[component]
pub fn IconBell(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"></path>
            <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"></path>
        </svg>
    }
}

#[component]
pub fn IconActivity(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>
        </svg>
    }
}

#[component]
pub fn IconCheckCircle(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
            <polyline points="22 4 12 14.01 9 11.01"></polyline>
        </svg>
    }
}

#[component]
pub fn IconAlertTriangle(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"></path>
            <line x1="12" y1="9" x2="12" y2="13"></line>
            <line x1="12" y1="17" x2="12.01" y2="17"></line>
        </svg>
    }
}

#[component]
pub fn IconAlertCircle(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
        </svg>
    }
}

#[component]
pub fn IconInfo(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="16" x2="12" y2="12"></line>
            <line x1="12" y1="8" x2="12.01" y2="8"></line>
        </svg>
    }
}

#[component]
pub fn IconStop(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
        </svg>
    }
}
