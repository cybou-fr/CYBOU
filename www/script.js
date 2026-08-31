// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

(() => {
  const header = document.querySelector('[data-header]');
  const menuButton = document.querySelector('[data-menu-button]');
  const mobileNav = document.querySelector('[data-mobile-nav]');
  const launcherButton = document.querySelector('[data-launcher-button]');
  const launcher = document.querySelector('[data-launcher]');
  const desktopStage = document.querySelector('[data-desktop-stage]');
  const clock = document.querySelector('[data-clock]');

  const translations = {
    en: {
      app_code: 'Agent Capsule',
      app_files: 'Files 2.0',
      app_ready: '{app} card focused on canvas',
      app_settings: 'Settings',
      app_web: 'Graph View',
      bp_ai_point1: '<strong>Ownership before intelligence:</strong> Model ≠ Identity, UI ≠ Mind, Attention ≠ Biography, and Proposal ≠ Authorization.',
      bp_ai_point2: '<strong>Durability before visibility:</strong> State is projected only after its owner commits it to the cryptographic Event1 ledger; consolidation never rewrites history.',
      bp_ai_point3: '<strong>Bounded degradation:</strong> Health1 publishes typed deficits and recovery progress; compound operations share one monotonic deadline.',
      bp_ai_point4: '<strong>Kernel-enforced agency:</strong> Autonomous agents run in Landlocked, cgroup-bounded capsules; actions require typed policy permits and independent re-observation.',
      bp_badge: 'Technical Blueprint · Sovereign AI Architecture',
      bp_banner_btn: 'Read Technical Blueprint',
      bp_banner_desc: 'The technical blueprint details the 14-daemon Mind runtime, kernel-enforced agent capsules, the Leptos/WASM spatial desktop, the Action1/Executor1 governance pipeline, and the cryptographic Event1 ledger.',
      bp_banner_h2_1: 'An AI-native operating environment,',
      bp_banner_h2_2: 'built one trusted layer at a time.',
      bp_banner_kicker: 'The Sovereign Architecture',
      bp_layer1_desc: 'Debian 13 packages, cgroups v2, Landlock LSM, and systemd user services deployed with explicit build and recovery gates. Debian is the production and integration authority.',
      bp_layer1_title: 'Layer 1: Reproducible Linux Body',
      bp_layer2_desc: '14 isolated systemd user daemons communicating over typed D-Bus contracts. Event1 is the single canonical Journal writer; Presence1 projects state without becoming a second owner; Meaning1 parses natural dialogue deterministically.',
      bp_layer2_title: 'Layer 2: Deterministic Mind Control Plane',
      bp_layer3_desc: 'A pure Rust/WebAssembly frontend compiled with Leptos, served over an authenticated Axum gateway. It provides 20+ specialized cards with sub-millisecond reactivity and zero compiler warnings.',
      bp_layer3_title: 'Layer 3: Living Canvas Spatial Desktop',
      bp_lead: 'Technical specification of the sovereign Mind control plane, kernel-enforced agent capsules, Leptos/WASM Living Canvas spatial desktop, and cryptographic Event1 ledger on Debian 13.',
      bp_lic1: '<strong>Code License:</strong> MIT License for all Rust workspace crates, daemons, web gateway, and deployment scripts.',
      bp_lic2: '<strong>Visual Assets:</strong> Creative Commons Attribution-ShareAlike 4.0 for logos, wallpapers, and desktop themes.',
      bp_lic3: '<strong>Trust Boundaries:</strong> Strict PAM authentication via cybou-authd; unprivileged host file access via per-UID Unix sockets.',
      bp_lic4: '<strong>Interface Boundary:</strong> Web gateway enforces strict CSP security headers, isolated sessions, and no-store policy.',
      bp_lic5: '<strong>Privacy & Erasure:</strong> Event envelopes carry sensitivity axes; crash-safe transitive cryptographic erasure safely destroys payloads on demand.',
      bp_lic6: '<strong>Zero Cloud Dependency:</strong> Operates 100% locally with zero mandatory cloud accounts, external API keys, or remote models.',
      bp_lic7: '<strong>Security Specifications:</strong> Version-controlled threat models, trust boundaries, and kernel isolation profiles in repository.',
      bp_meta_foundation: 'Foundation:',
      bp_meta_foundation_v: 'Debian 13, 100% Rust workspace, cgroups v2, Landlock LSM',
      bp_meta_interface: 'Interface:',
      bp_meta_interface_v: 'Living Canvas (Rust / Leptos / WebAssembly)',
      bp_meta_status: 'Status:',
      bp_meta_status_v: 'Mind, Capsules, Shell, Host Files, Learning & Storage Live',
      bp_meta_target: 'Target: Debian 13 (Trixie)',
      bp_meta_updated: 'Updated:',
      bp_meta_updated_v: 'August 2026',
      bp_meta_version: 'Pre-Release 0.1 · 100% Rust',
      bp_s1_card1_text: '14 decoupled D-Bus micro-daemons (<code>org.cybou.Mind.*</code>) managing identity, biography, intentions, context, health, and lifecycle with zero probabilistic prompt collapse.',
      bp_s1_card1_title: 'Deterministic Mind Control Plane',
      bp_s1_card2_text: 'Agent capsules isolated via Linux cgroups v2, Landlock LSM, mount namespaces, and mediated egress. Agents formulate typed action proposals rather than running raw shell commands.',
      bp_s1_card2_title: 'Kernel-Enforced Agent Capsules',
      bp_s1_card3_text: 'GPU-accelerated Leptos/WASM spatial UI delivering 20+ responsive cards (Shell, Files, Cognitive Graph, System Monitor, Notes) over an authenticated local Axum gateway.',
      bp_s1_card3_title: 'Living Canvas Spatial Desktop',
      bp_s1_card4_text: 'Rigorous governance pipeline: Action Proposal &rarr; Policy Permit &rarr; Executor1 &rarr; Independent Telemetry Re-Observation to verify real-world outcomes.',
      bp_s1_card4_title: 'Two-Phase Action1 & Executor1',
      bp_s3_p1: '<strong>Pure Rust/WASM Frontend:</strong> Zero hand-written JavaScript; all 20+ cards compile into a single WebAssembly binary with zero compiler warnings.',
      bp_s3_p2: '<strong>Authenticated Web Gateway:</strong> <code>cybou-web-gateway</code> bridges browser sessions to local D-Bus daemons via typed JSON-RPC and SSE event streams.',
      bp_s3_p3: '<strong>Dual Shell Engine:</strong> Supports bounded safe demonstration shells (<code>cybou-jailfs</code>) and interactive unprivileged host terminals (<code>cybou-shelld</code>).',
      bp_s3_p4: '<strong>Host Filesystem Integration:</strong> Multi-panel file manager powered by <code>cybou-host-filesd</code> operating over per-UID unprivileged Unix sockets.',
      bp_s4_p1: '<strong>Agent Capsules:</strong> External agents (e.g. OpenCode ACP) run inside Linux Bubblewrap/Landlock sandboxes with private filesystem namespaces and strict CPU/memory limits.',
      bp_s4_p2: '<strong>Model Brokerage:</strong> Models are leased through a local broker with sensitivity-axis gating and spending ceilings; raw API keys never enter agent capsules.',
      bp_s4_p3: '<strong>Two-Phase Action Pipeline:</strong> Action Proposal &rarr; Policy Evaluation &rarr; Single-Use Permit &rarr; Executor1 &rarr; Telemetry Re-Observation.',
      bp_s4_p4: '<strong>Cryptographic Journal:</strong> Every accepted observation and action outcome is signed and appended to a SHA-256 hash-chained ledger.',
      bp_sec1_lead: 'Cybou is an agent-native operating environment and cognitive control plane built entirely in Rust for Debian 13 Linux. It continuously observes its own state, diagnoses system anomalies, and executes governed actions through kernel-isolated agent capsules and a cryptographic event ledger.',
      bp_sec1_p1: 'CYBOU is a sovereign, agent-native operating environment engineered in 100% Rust. It establishes a deterministic cognitive control plane (Mind) that owns durable biography, identity, commitments, prediction, attention, health, and recovery, paired with an unprivileged Living Canvas desktop. AI models and agents operate as untrusted guests inside kernel-enforced capsules.',
      bp_sec1_title: '1. Executive Architecture Summary',
      bp_sec2_lead: 'Mind runs as 14 process-isolated systemd user services communicating over the session D-Bus bus:',
      bp_sec2_p1: 'CYBOU enforces strict separation between Body, Mind, and Presence. Durable state belongs to explicit daemon owners; user interfaces are pure projection boundaries.',
      bp_sec2_title: '2. Process Topology & Daemon Architecture',
      bp_sec3_lead: 'Living Canvas provides an infinite 2D spatial workspace built entirely with Leptos and WebAssembly:',
      bp_sec3_p1: 'The cognitive substrate ensures inspectability and safety by enforcing fundamental invariants across all components:',
      bp_sec3_title: '3. Living Canvas Spatial Desktop Architecture',
      bp_sec4_lead: 'Cybou\'s governance model guarantees that autonomous agents can never execute unverified or destructive actions:',
      bp_sec4_p1: 'Platform capabilities are continuously verified against 88+ automated test suites and live production deployments on Debian 13.',
      bp_sec4_title: '4. Governance, Agent Capsules & Action Pipeline',
      bp_sec5_lead: 'Implementation milestones verified on Debian 13 Linux:',
      bp_sec5_p1: 'CYBOU is free open-source software built for sovereignty, privacy, and zero cloud dependency:',
      bp_sec5_title: '5. Implementation Evidence & Live Milestones',
      bp_sec6_title: '6. Frequently Asked Questions',
      bp_sources_lead: 'This web specification summarizes the implementation. Authoritative technical references are maintained in the repository:',
      bp_sources_title: 'Canonical Sources & Verification Authority',
      bp_src1: '— authoritative implemented boundary and known limitations.',
      bp_src2: '— process topology, data ownership, and cognitive invariants.',
      bp_src3: '— test evidence, fault-injection, and KVM live-bus verification.',
      bp_src4: '— detailed implementation log and current active milestones.',
      bp_stack1: '<strong>Base System:</strong> Debian 13 Linux (trixie) with systemd user manager, cgroups v2 resource controllers, Landlock LSM, and bubblewrap isolation.',
      bp_stack10: '<strong>Zero Cloud Lock-in:</strong> Fully local-sufficient operation with zero mandatory cloud dependencies and zero remote telemetry.',
      bp_stack2: '<strong>Rust Workspace:</strong> 35 modular crates covering protocol contracts, cryptographic storage, D-Bus fabric, agent runtime, web gateway, and the Living Canvas WASM desktop.',
      bp_stack3: '<strong>Web Gateway:</strong> High-performance Axum HTTP/WebSocket gateway with PAM authentication, unprivileged per-UID host file routing, and cryptographic event streaming.',
      bp_stack4: '<strong>Agent Capsules:</strong> Isolated guest execution environments with declarative operator profiles, spend ceilings, CPU/RAM limits, and brokered model egress.',
      bp_stack5: '<strong>Living Canvas:</strong> Spatial infinite-plane desktop UI written in Leptos/WASM with 20+ responsive cards, snap guides, and sub-millisecond interaction.',
      bp_stack6: '<strong>Cryptographic Ledger:</strong> Append-only SQLite v3 journal with SHA-256 hash chains, causal provenance tracking, and crash-safe transitive erasure.',
      bp_stack7: '<strong>Mind Control Plane:</strong> 14 process-isolated D-Bus micro-daemons handling epistemics, meaning, associative context, prediction, health, and lifecycle.',
      bp_stack8: '<strong>Two-Phase Governance:</strong> Typed action proposals evaluated against standing policy, generating opaque single-use permits for Executor1.',
      bp_stack9: '<strong>Sovereign Persistence:</strong> Atomic local storage for notes, contacts, calendar events, and lifelong learning artifact lineages.',
      bp_stack_title: 'Core System Stack',
      bp_title: 'CYBOU Technical Blueprint & Architecture',
      bp_topology_title: 'Process and ownership topology',
      btn_back_home: '← Back to home',
      btn_print_pdf: 'Export to PDF / Print',
      btn_top: 'Back to top',
      btn_view_blueprint: 'Blueprint',
      btn_view_state: 'View Current State',
      concept_badge: 'Living Canvas Preview',
      d_actiond: 'Action1',
      d_actiond_r: 'Action proposal validation, policy criticism, single-use execution permits',
      d_actiond_s: 'Policy store',
      d_agentd: 'Agent1',
      d_agentd_r: 'Agent capsule supervisor, Landlock/cgroups lifecycle management',
      d_agentd_s: 'Capsule descriptors',
      d_contextd: 'Context1',
      d_contextd_r: 'Associative memory retrieval, temporal decay, contextual activation',
      d_contextd_s: 'Context graph',
      d_epistemicd: 'Epistemic1',
      d_epistemicd_r: 'Epistemic claims, evidence verification, grounded belief management',
      d_epistemicd_s: 'Epistemic store',
      d_eventd: 'Event1',
      d_eventd_r: 'Canonical Journal writer, SHA-256 hash chaining, causal ordering',
      d_eventd_s: 'SQLite v3 (<code>journal.db</code>)',
      d_healthd: 'Health1',
      d_healthd_r: 'Subsystem health, homeostatic monitoring, capability deficit graphs',
      d_healthd_s: 'Health snapshots',
      d_identityd: 'Identity1',
      d_identityd_r: 'Logical session continuity, subject continuity, biography tracking',
      d_identityd_s: 'Runtime marker & Journal',
      d_intentiond: 'Intention1',
      d_intentiond_r: 'Commitment tracking, obligations, and terminal intention state',
      d_intentiond_s: 'Journal records',
      d_lifecycled: 'Lifecycle1',
      d_lifecycled_r: 'Evidence-bound maintenance runs, automatic sleep/wake scheduling',
      d_lifecycled_s: 'Lifecycle records',
      d_meaningd: 'Meaning1',
      d_meaningd_r: 'Cognitive act parser, natural dialogue intent extraction',
      d_meaningd_s: 'Grammar rules',
      d_predictord: 'Predictor1',
      d_predictord_r: 'Statistical calibration, horizon prediction, forecast validation',
      d_predictord_s: 'Calibration models',
      d_presenced: 'Presence1',
      d_presenced_r: 'Aggregated presentation snapshot, command gating, UI signals',
      d_presenced_s: 'Projections cache',
      d_selfd: 'Self1',
      d_selfd_r: 'Autobiographical self-model, capability projection, self-assessment',
      d_selfd_s: 'Derived self state',
      d_telemetryd: 'Telemetry1',
      d_telemetryd_r: 'Host metrics, service health, socket states, diagnostic readings',
      d_telemetryd_s: 'Telemetry streams',
      d_workspaced: 'Workspace1',
      d_workspaced_r: 'Transient bounded attention, active cards, focal context',
      d_workspaced_s: 'In-memory attention ring',
      design_sub: 'An infinite 2D spatial plane where cards, decks, relations, and system insight live together without overlapping window chaos.',
      desktop_stable: 'Developer Substrate',
      desktop_sub: 'A sovereign cognitive runtime. Built for engineers.',
      desktop_welcome: 'Welcome to CYBOU',
      evidence_source: 'Source Repository',
      evidence_source_desc: 'Explore the code, ADRs, and open-source history on GitHub',
      evidence_state: 'Current State',
      evidence_state_desc: 'Implemented architecture, active capabilities, and known limitations',
      evidence_tests: 'Testing Evidence',
      evidence_tests_desc: '88+ test suites passing with 100% success',
      exp_h2_1: 'Continuous observation.',
      exp_h2_2: 'Grounded in cryptographic evidence.',
      exp_lead: 'CPU load, memory pressure, I/O latency, filesystem inodes, open descriptors, and declared systemd services stream continuously into a tamper-evident SHA-256 hash-chain Journal. Findings carry verifiable evidence rather than probabilistic LLM assertions.',
      faq_a1: 'Cybou is an open-source, sovereign agent-native operating environment and spatial desktop in active development, built in 100% Rust for Debian 13 Linux. A persistent, deterministic local control plane called Mind continuously observes and governs the host machine, while autonomous agent capsules, models, and tools remain strictly sandboxed and replaceable.',
      faq_a2: 'No, Cybou is currently in active pre-release developer preview. The core Mind control plane, cryptographic ledger, Living Canvas desktop, and kernel-isolated agent capsules are fully operational in development and laboratory environments. It is intended for developers, researchers, and sovereign infrastructure builders.',
      faq_a3: 'A model is not Mind. An agent is not Mind. Tool access is not permission. Conventional AI assistants collapse memory, identity, execution, and security into a single probabilistic LLM prompt. Cybou establishes an explicit zero-trust architecture: memory, identity, health, lifecycle, and recovery are separate D-Bus micro-daemons. AI agents run in kernel-enforced sandboxes (cgroups v2, Landlock LSM) and can only propose actions that pass strict policy validation.',
      faq_a4: 'No. Cybou is local-sufficient by design: core cognition, identity, biography, meaning parsing, and the Living Canvas desktop require zero cloud connectivity, no GPU accelerator, and no external API keys. It collects and sends zero remote telemetry.',
      faq_a5: 'Living Canvas is a high-performance spatial desktop interface built entirely with Leptos and WebAssembly (Rust). It communicates over a secure Axum web gateway with typed JSON and real-time event streams, providing 20+ specialized cards including a sandboxed shell, unprivileged host file manager, cognitive graph, and system telemetry.',
      faq_a6: 'Cybou enforces a strict two-phase Action1 / Executor1 governance pipeline: agents or users propose typed actions; Mind evaluates governance policy against empirical evidence and issues an opaque single-use permit; cybou-executord carries out the physical effect; and telemetry independently re-observes the true outcome. If telemetry does not confirm success, the action is marked failed and committed to the hash-chained Journal.',
      faq_a7: 'Cybou code and documentation are licensed under the MIT License. Original visual assets are licensed under Creative Commons CC BY-SA 4.0. The repository is 100% compliant with the REUSE specification.',
      faq_h2_1: 'Frequently asked',
      faq_h2_2: 'questions.',
      faq_q1: 'What is Cybou?',
      faq_q2: 'Is Cybou production-ready?',
      faq_q3: 'How is that different from adding an AI chatbot to an OS?',
      faq_q4: 'Does Cybou require cloud services or send telemetry?',
      faq_q5: 'What is the Living Canvas interface built with?',
      faq_q6: 'How are actions and agent executions governed?',
      faq_q7: 'What open-source licenses apply?',
      feat1_desc: 'A GPU-accelerated Leptos/WASM spatial interface delivering 20+ specialized cards (Shell, Files, Cognitive Graph, System Monitor, Notes) with sub-millisecond responsiveness on an infinite 2D canvas.',
      feat1_title: 'Living Canvas Spatial Desktop',
      feat2_desc: 'AI agents execute in strictly unprivileged sandboxes bounded by Linux cgroups v2, Landlock LSM, mount namespaces, and mediated egress under declarative operator profiles.',
      feat2_title: 'Kernel-Enforced Agent Capsules',
      feat3_desc: 'Append-only SQLite v3 journal with SHA-256 hash chaining, causal ordering, and crash-safe transitive cryptographic erasure (ADR-0028).',
      feat3_title: 'Cryptographic Event1 Ledger',
      footer_sub: 'Sovereign Agent-Native Environment · 100% Rust & WebAssembly · Debian 13.',
      found_h2_1: 'Typed Proposal.',
      found_h2_2: 'Governed Execution.',
      found_lead: 'Cybou strictly decouples proposal from execution. Autonomous agents and users formulate typed action proposals. Mind evaluates them against standing security policy and empirical evidence, issuing single-use permits to Executor1. True physical outcomes are independently re-observed by telemetry.',
      gen145_desc: 'Cryptographic ledger · 46k+ events',
      gen146_desc: 'Kernel sandboxing & capsules · active',
      gen147_desc: 'Active · Verified Mind Control Plane',
      gen_active: 'Active & Verified',
      gen_aug2: 'Live',
      gen_console_title: 'Substrate telemetry',
      gen_status: 'Mind substrate active & verified',
      gen_yesterday: 'Verified',
      hero_btn_blueprint: 'Read Technical Blueprint',
      hero_btn_explore: 'Explore Code & Architecture',
      hero_eyebrow: 'Developer Preview · 100% Rust & WebAssembly · Debian 13',
      hero_h1_1: 'The Sovereign, Agent-Native',
      hero_h1_2: 'Cognitive Operating Environment.',
      hero_lead: 'Cybou is an open-source, agent-native operating environment and spatial canvas in active development. Built entirely in pure Rust for Debian 13 Linux, it establishes a deterministic local Mind control plane, kernel-sandboxed agent capsules, two-phase action governance, and a tamper-evident cryptographic event ledger. 100% local-sufficient with zero cloud dependencies.',
      kicker_design: '04 · Spatial Model',
      kicker_faq: '07 · Architecture FAQ',
      kicker_foundation: '02 · Governance',
      kicker_interface: '01 · Interface & Substrate',
      kicker_principles: '05 · Core Invariants',
      kicker_progress: '03 · Engineering Progress',
      kicker_roadmap: '06 · Roadmap & State',
      label_palette: 'Theme',
      label_symbol: 'Spatial Primitive',
      label_typography: 'Performance',
      launcher_search: 'Search spatial cards & daemons',
      mark_desc: 'Cards and decks snap to dynamic layout guides with instant zoom, collapse, and multi-mode arrangement.',
      metric_contrast: 'Pure Rust workspace with strict safety guarantees and zero synthetic fixtures in production',
      metric_gate_a: '14 Isolated Mind micro-daemons with typed D-Bus interfaces',
      metric_tasks: 'Automated CI & deployment gates: format, clippy, unit tests, and live-bus KVM validation',
      metric_v_crates: '35 Rust Crates',
      metric_v_services: '14 Mind Daemons',
      metric_v_tests: '88+ Test Suites (100% Pass)',
      nav_blueprint: 'Blueprint',
      nav_design: 'Spatial Canvas',
      nav_experience: 'Interface',
      nav_faq: 'FAQ',
      nav_foundation: 'Foundation',
      nav_github: 'GitHub',
      nav_partners: 'Partners',
      nav_partners_footer: 'Partners & Sponsorship',
      nav_progress: 'Progress',
      nav_roadmap: 'Roadmap',
      p1_head: 'Typed Proposal',
      p1_text: 'Closed schema operations from a verified protocol set, never raw unmonitored shell strings.',
      p2_head: 'Policy Evaluation',
      p2_text: 'Evaluated against empirical findings, operator risk rules, and capability budgets.',
      p3_head: 'Independent Outcome',
      p3_text: 'Physical effect is confirmed by telemetry re-observation before commitment to history.',
      palette_name: 'Mineral Dark · Aurora Mint',
      part_badge: 'Partnership & Support Hub',
      part_contact_head: 'Official Contact',
      part_contact_text: 'For enterprise inquiries, press, security disclosures, Linux distribution collaboration, or hardware enablement:',
      part_crypto_head: 'Crypto Donations',
      part_crypto_text: 'Support Cybou directly via cryptocurrency:',
      part_donate_head: 'Support & Donations',
      part_donate_text: 'Cybou is an independent open-source project. Your support funds reproducible builds, the Rust/WebAssembly Living Canvas desktop, Mind control plane engineering, and fault-recovery testing on Debian 13.',
      part_lead: 'Get in touch with the Cybou team, explore hardware & software partnerships, or support Cybou’s independent open-source development.',
      part_title: 'Partners, Contact & Donations',
      pr1_desc: 'AI models are replaceable statistical faculties. Memory, identity, and system invariants stay persistent and local.',
      pr1_head: 'Model ≠ Mind',
      pr2_desc: 'Agents operate inside Linux cgroups v2, Landlock LSM, and namespace sandboxes with explicit resource budgets.',
      pr2_head: 'Kernel-Bounded Autonomy',
      pr3_desc: 'Zero telemetry, zero remote tracking, and 100% offline local sufficiency.',
      pr3_head: 'Private by Construction',
      pr4_desc: 'Every cognitive belief is backed by verifiable host telemetry and cryptographic ledger evidence.',
      pr4_head: 'Truth over Hallucination',
      princ_h2_1: 'Replaceable where possible.',
      princ_h2_2: 'Governed where required.',
      prog_h2_1: 'Rigorous Engineering.',
      prog_h2_2: 'Open-Source Pre-Release.',
      prog_lead: 'The platform integrates 14 isolated D-Bus micro-daemons, PAM credential separation, per-UID unprivileged host file sockets, sandboxed shells, lifelong learning gates, and persistent sovereign storage. Tested across 88+ automated test suites.',
      rm1_sub: '14 D-Bus daemons · Durable history · Epistemics · Transitive Erasure · Identity continuity',
      rm1_title: 'Mind Cognitive Substrate',
      rm2_sub: 'Leptos/WASM · Infinite spatial canvas · 20+ specialized cards · Responsive layout',
      rm2_title: 'Living Canvas Spatial UI',
      rm3_sub: 'Telemetry1 · Cognitive Graph · Natural dialogue parser · Epistemic findings',
      rm3_title: 'Observation & Dialogue Meaning',
      rm4_sub: 'Bubblewrap/Landlock capsules · Action1/Executor1 pipeline · Lifelong learning persistence',
      rm4_title: 'Agent Capsules & Sovereign Storage',
      rm_complete: 'Implemented & Tested',
      rm_inprogress: 'In Active Development',
      road_h2_1: 'Sovereign foundation.',
      road_h2_2: 'Active development.',
      row_body_bound: 'Bounded transient telemetry over Linux /proc and statvfs, hypothesis generation, and robust statistics',
      row_body_cap: 'Body Observation & Telemetry',
      row_body_ev: 'Live observation of 46,300+ cryptographic event contributions on Debian 13',
      row_m0_bound: 'Locked Cargo workspace, strict formatting, lints, metadata, and licensing gates',
      row_m0_cap: 'Reproducible Rust Baseline',
      row_m0_ev: 'Automated CI gates across all 35 workspace crates',
      row_m14_bound: 'Single Event1 writer, isolated D-Bus services, and Presence1 projection daemon',
      row_m14_cap: 'Accepted Memory & Isolated Daemons',
      row_m14_ev: 'D-Bus session bus tests, event ordering, and VM integration gates',
      row_m5_bound: 'Persistent state, Lifecycle1, deterministic owner effects, and reboot recovery',
      row_m5_cap: 'Continuity & Consolidation Lifecycle',
      row_m5_ev: 'Lifecycle continuity across real host reboots',
      row_m6_bound: 'Capability dependency graph, Health1, homeostatic scheduling, and degraded UI contracts',
      row_m6_cap: 'Health, Homeostasis & Recovery',
      row_m6_ev: 'Recovery boundary and process fault injection matrices',
      row_m7_bound: 'Perception, epistemics, Journal v3 transitive erasure, sensitivity, and associative memory',
      row_m7_cap: 'Grounded Cognition & Epistemic Truth',
      row_m7_ev: 'Scale budgets, disclosure filters, and cryptographic hash chain verification',
      row_m8_bound: 'Replaceable model broker with typed context contracts and ACP agent packs',
      row_m8_cap: 'Model Brokerage & ACP Agent Integration',
      row_m8_ev: 'OpenCode ACP agent pack integration and keyless model leases verified',
      row_m913_bound: 'Landlock/cgroups capsules, two-phase Action1/Executor1 governance, and learning persistence',
      row_m913_cap: 'Agent Capsules, Action1 & Learning Persistence',
      row_m913_ev: 'Agent capsule launches and empirical induction gates verified live',
      row_p67_bound: 'Deterministic natural dialogue parsing, structured ResponsePlan generation, and bounded RPC deadlines',
      row_p67_cap: 'Presence & Meaning Boundary',
      row_p67_ev: 'Meaning walkthrough tests and deterministic cognitive act parsing gates',
      row_w01_bound: 'Axum web gateway with PAM authentication, unprivileged per-UID host files socket, and sandboxed shell',
      row_w01_cap: 'Living Canvas & Web Gateway',
      row_w01_ev: 'Native tests, WASM build with 0 compiler warnings, and live browser gates',
      row_w2_bound: 'Infinite spatial canvas with Shell, Files, Cognitive Graph, Monitor, Notes, Calendar',
      row_w2_cap: 'Living Canvas Spatial Desktop (20+ Cards)',
      row_w2_ev: 'Verified on Debian 13 VPS with sub-millisecond response times',
      skip_link: 'Skip to content',
      status_development: 'Active Substrate',
      status_runtime: 'Verified Baseline',
      status_verified: 'Operational',
      tag_ai: 'Epistemic Truth, Not Hallucinations',
      tag_built_with: '100% Rust · WebAssembly',
      tag_done: 'DONE',
      tag_inprogress: 'ACTIVE',
      tag_planned: 'PLANNED',
      tag_telemetry: 'Zero Remote Telemetry',
      tag_unplugged: '100% Local-Sufficient',
      th_boundary: 'Architecture Boundary',
      th_bus: 'D-Bus Interface',
      th_capability: 'Capability & Scope',
      th_daemon: 'Daemon Process',
      th_evidence: 'Verification Evidence',
      th_milestone: 'Milestone',
      th_responsibility: 'Owned Responsibility',
      th_state: 'Persisted State',
      th_status: 'Status',
      toc_architecture: 'Architecture',
      toc_faq: 'FAQ',
      toc_implementation: 'Implementation',
      toc_safety: 'Safety boundaries',
      toc_security: 'Security & licensing',
      toc_vision: 'Vision',
      type_desc: 'Lightweight, responsive WebAssembly rendering ensuring distraction-free spatial operation.',
      type_head: 'Zero Compiler Warnings · Leptos WASM',
      visual_caption: 'Living Canvas spatial environment — infinite canvas with 20+ responsive cards',
      ws_active: 'Spatial Workspace {n} active',
    },
    fr: {
      app_code: 'Capsule d\'Agent',
      app_files: 'Fichiers 2.0',
      app_ready: 'Carte {app} active sur le canevas',
      app_settings: 'Paramètres',
      app_web: 'Graphe Cognitif',
      bp_ai_point1: '<strong>Propriété avant intelligence :</strong> Modèle ≠ Identité, UI ≠ Mind, Attention ≠ Biographie, et Proposition ≠ Autorisation.',
      bp_ai_point2: '<strong>Durabilité avant visibilité :</strong> L’état n’est projeté qu’après enregistrement par son propriétaire dans le registre cryptographique Event1 ; la consolidation ne réécrit jamais l’histoire.',
      bp_ai_point3: '<strong>Dégradation bornée :</strong> Health1 publie les déficits typés et l’avancement des réparations ; les opérations composées partagent un délai d’expiration monotone.',
      bp_ai_point4: '<strong>Agencement imposé par le noyau :</strong> Les agents autonomes tournent dans des capsules Landlocked délimitées par cgroups ; les actions exigent des permis de politique typés et une ré-observation indépendante.',
      bp_badge: 'Blueprint technique · Architecture IA souveraine',
      bp_banner_btn: 'Lire le plan technique',
      bp_banner_desc: 'Le plan technique détaille le runtime Mind à 14 démons, les capsules d\'agents isolées par le noyau, le bureau spatial Leptos/WASM, le pipeline de gouvernance Action1/Executor1 et le registre cryptographique Event1.',
      bp_banner_h2_1: 'Un environnement opérationnel orienté agents,',
      bp_banner_h2_2: 'construit une couche de confiance à la fois.',
      bp_banner_kicker: 'L\'Architecture Souveraine',
      bp_layer1_desc: 'Paquets Debian 13, cgroups v2, Landlock LSM et services utilisateur systemd déployés avec des portes explicites de build et de reprise. Debian est l’autorité d’intégration et de production.',
      bp_layer1_title: 'Couche 1 : Corps Linux reproductible',
      bp_layer2_desc: '14 démons utilisateur systemd isolés communiquant via des contrats D-Bus typés. Event1 est l’unique rédacteur du Journal ; Presence1 projette l’état sans être un second propriétaire ; Meaning1 analyse le dialogue de manière déterministe.',
      bp_layer2_title: 'Couche 2 : Plan de contrôle déterministe Mind',
      bp_layer3_desc: 'Un frontend pur Rust/WebAssembly compilé en Leptos, servi via une passerelle Axum authentifiée. Il offre plus de 20 cartes spécialisées avec une réactivité sub-milliseconde et zéro avertissement.',
      bp_layer3_title: 'Couche 3 : Bureau spatial Living Canvas',
      bp_lead: 'Spécification technique du plan de contrôle Mind, des capsules d\'agents isolées par le noyau, du bureau spatial Living Canvas en Leptos/WASM et du registre cryptographique Event1 sous Debian 13.',
      bp_lic1: '<strong>Licence du Code :</strong> Licence MIT pour tous les crates Rust, les démons, la passerelle web et les scripts de déploiement.',
      bp_lic2: '<strong>Ressources Visuelles :</strong> Creative Commons Attribution-ShareAlike 4.0 pour les logos, fonds d\'écran et thèmes.',
      bp_lic3: '<strong>Frontières de Confiance :</strong> Authentification PAM stricte via cybou-authd ; accès aux fichiers hôte non privilégié via sockets UID.',
      bp_lic4: '<strong>Frontière d\'Interface :</strong> La passerelle web applique des en-têtes CSP stricts, des sessions isolées et une politique no-store.',
      bp_lic5: '<strong>Confidentialité & Effacement :</strong> Les enveloppes portent des axes de sensibilité ; l\'effacement cryptographique transitif détruit les données à la demande.',
      bp_lic6: '<strong>Zéro Dépendance Cloud :</strong> Fonctionne 100% localement sans compte cloud obligatoire, clé API externe ou modèle distant.',
      bp_lic7: '<strong>Spécifications de Sécurité :</strong> Modèles de menaces, frontières de confiance et profils d\'isolation du noyau sous contrôle de version.',
      bp_meta_foundation: 'Fondation :',
      bp_meta_foundation_v: 'Debian 13, espace de travail 100 % Rust, cgroups v2, Landlock LSM',
      bp_meta_interface: 'Interface :',
      bp_meta_interface_v: 'Living Canvas (Rust / Leptos / WebAssembly)',
      bp_meta_status: 'Statut :',
      bp_meta_status_v: 'Mind, Capsules, Shell, Fichiers hôte, Apprentissage & Stockage en direct',
      bp_meta_target: 'Cible : Debian 13 (Trixie)',
      bp_meta_updated: 'Mis à jour :',
      bp_meta_updated_v: 'Août 2026',
      bp_meta_version: 'Pré-version 0.1 · 100% Rust',
      bp_s1_card1_text: '14 micro-démons D-Bus découplés (<code>org.cybou.Mind.*</code>) gérant l\'identité, la biographie, les intentions, le contexte et la santé sans effondrement de prompt probabiliste.',
      bp_s1_card1_title: 'Plan de Contrôle Mind Déterministe',
      bp_s1_card2_text: 'Capsules isolées via cgroups v2, Landlock LSM et espaces de noms privés. Les agents formulent des propositions d\'actions typées au lieu de lancer des commandes shell brutes.',
      bp_s1_card2_title: 'Capsules d\'Agents Isolées par le Noyau',
      bp_s1_card3_text: 'Interface spatiale Leptos/WASM accélérée par GPU offrant 20+ cartes réactives (Terminal, Fichiers, Graphe, Moniteur, Notes) via une passerelle locale Axum authentifiée.',
      bp_s1_card3_title: 'Bureau Spatial Living Canvas',
      bp_s1_card4_text: 'Pipeline rigoureux : Proposition d\'Action &rarr; Permis de Sécurité &rarr; Executor1 &rarr; Réobservation Télémétrique Indépendante.',
      bp_s1_card4_title: 'Gouvernance Action1 & Executor1',
      bp_s3_p1: '<strong>Frontend Pur Rust/WASM :</strong> Zéro JavaScript écrit à la main ; les 20+ cartes compilent en un binaire WebAssembly unique sans avertissement.',
      bp_s3_p2: '<strong>Passerelle Web Authentifiée :</strong> <code>cybou-web-gateway</code> relie les sessions du navigateur aux démons D-Bus locaux en JSON-RPC et flux SSE.',
      bp_s3_p3: '<strong>Double Moteur de Shell :</strong> Prise en charge de shells de démonstration confinés (<code>cybou-jailfs</code>) et de terminaux hôte interactifs non privilégiés (<code>cybou-shelld</code>).',
      bp_s3_p4: '<strong>Intégration du Système de Fichiers :</strong> Gestionnaire de fichiers multi-panneaux propulsé par <code>cybou-host-filesd</code> via des sockets Unix par UID.',
      bp_s4_p1: '<strong>Capsules d\'Agents :</strong> Les agents externes (ex. OpenCode ACP) s\'exécutent dans des bacs à sable Linux Bubblewrap/Landlock avec namespaces privés et limites CPU/RAM.',
      bp_s4_p2: '<strong>Courtage de Modèles :</strong> Les modèles sont loués via un courtier local avec axes de sensibilité et plafonds budgétaires ; les clés d\'API ne pénètrent jamais dans les capsules.',
      bp_s4_p3: '<strong>Pipeline d\'Actions à Deux Phases :</strong> Proposition d\'Action &rarr; Évaluation de Politique &rarr; Permis à Usage Unique &rarr; Executor1 &rarr; Réobservation Télémétrique.',
      bp_s4_p4: '<strong>Journal Cryptographique :</strong> Chaque observation et résultat accepté est signé et consigné dans un registre immuable chaîné par SHA-256.',
      bp_sec1_lead: 'Cybou est un environnement opérationnel orienté agents et un plan de contrôle cognitif conçu entièrement en Rust pour Debian 13. Il observe en continu son propre état, diagnostique les anomalies et exécute des actions gouvernées via des capsules d\'agents isolées et un registre d\'événements cryptographique.',
      bp_sec1_p1: 'CYBOU est un environnement d’exploitation souverain et natif pour agents conçu à 100 % en Rust. Il établit un plan de contrôle cognitif déterministe (Mind) qui détient la biographie, l’identité, les engagements, la prédiction, l’attention, la santé et la reprise, couplé à un bureau spatial Living Canvas non privilégié. Les modèles et agents IA y opèrent comme des invités non fiables au sein de capsules délimitées par le noyau.',
      bp_sec1_title: '1. Résumé de l\'Architecture',
      bp_sec2_lead: 'Mind s\'exécute sous forme de 14 services utilisateur systemd isolés communiquant sur le bus D-Bus de session :',
      bp_sec2_p1: 'CYBOU impose une séparation stricte entre le Corps, Mind et Presence. L’état durable appartient à des démons propriétaires explicites ; les interfaces sont de pures frontières de projection.',
      bp_sec2_title: '2. Topologie des Processus & Architecture des Démons',
      bp_sec3_lead: 'Living Canvas propose un espace de travail 2D infini conçu entièrement en Leptos et WebAssembly :',
      bp_sec3_p1: 'Le substrat cognitif garantit l’inspectabilité et la sécurité en imposant des invariants fondamentaux sur l’ensemble des composants :',
      bp_sec3_title: '3. Architecture du Bureau Spatial Living Canvas',
      bp_sec4_lead: 'Le modèle de gouvernance de Cybou garantit que les agents autonomes ne peuvent exécuter aucune action non vérifiée :',
      bp_sec4_p1: 'Les capacités de la plateforme sont continuellement vérifiées face à plus de 88 suites de tests automatisées et des déploiements réels en production sur Debian 13.',
      bp_sec4_title: '4. Gouvernance, Capsules d\'Agents & Pipeline d\'Actions',
      bp_sec5_lead: 'Jalons d\'ingénierie vérifiés sur Linux Debian 13 :',
      bp_sec5_p1: 'CYBOU est un logiciel libre conçu pour la souveraineté, la confidentialité et l’indépendance totale vis-à-vis du cloud :',
      bp_sec5_title: '5. Preuves d\'Implémentation & Jalons Validés',
      bp_sec6_title: '6. Foire Aux Questions',
      bp_sources_lead: 'Ce document résume l\'implémentation. Les références techniques faisant autorité sont maintenues dans le dépôt :',
      bp_sources_title: 'Sources Canoniques & Autorité de Vérification',
      bp_src1: '— frontière implémentée faisant autorité et limites connues.',
      bp_src2: '— topologie des processus, propriété des données et invariants cognitifs.',
      bp_src3: '— preuves de tests, injection de fautes et validation KVM en direct.',
      bp_src4: '— journal d\'implémentation détaillé et jalons actifs.',
      bp_stack1: '<strong>Système de base :</strong> Debian 13 Linux (trixie) avec gestionnaire utilisateur systemd, contrôleurs de ressources cgroups v2, Landlock LSM et isolation bubblewrap.',
      bp_stack10: '<strong>Zéro verrouillage cloud :</strong> Fonctionnement 100 % local-suffisant sans dépendance cloud obligatoire et sans télémétrie distante.',
      bp_stack2: '<strong>Espace Rust :</strong> 35 crates modulaires couvrant protocoles, stockage cryptographique, fabric D-Bus, runtime d’agents, passerelle web et bureau WASM Living Canvas.',
      bp_stack3: '<strong>Passerelle Web :</strong> Passerelle Axum HTTP/WebSocket haute performance avec authentification PAM, routage de fichiers hôte non privilégié par UID et flux d’événements cryptographiques.',
      bp_stack4: '<strong>Capsules d’agents :</strong> Environnements d’exécution isolés avec profils déclaratifs, plafonds de dépenses, limites CPU/RAM et egress de modèles médié.',
      bp_stack5: '<strong>Living Canvas :</strong> Bureau spatial infini en Leptos/WASM avec plus de 20 cartes réactives, guides magnétiques et interactions sub-millisecondes.',
      bp_stack6: '<strong>Registre cryptographique :</strong> Journal SQLite v3 append-only avec chaînes de hachage SHA-256, traçabilité causale et effacement cryptographique transitif.',
      bp_stack7: '<strong>Plan de contrôle Mind :</strong> 14 micro-démons D-Bus gérant l’épistémique, le sens, le contexte associatif, les prédictions, la santé et le cycle de vie.',
      bp_stack8: '<strong>Gouvernance à 2 phases :</strong> Propositions d’action typées évaluées face aux politiques, générant des permis à usage unique pour Executor1.',
      bp_stack9: '<strong>Persistance souveraine :</strong> Stockage local atomique pour les notes, contacts, événements de calendrier et lignées d’artéfacts d’apprentissage.',
      bp_stack_title: 'Pile système centrale',
      bp_title: 'Plan Technique & Architecture CYBOU',
      bp_topology_title: 'Topologie des processus et de la propriété',
      btn_back_home: '← Retour à l’accueil',
      btn_print_pdf: 'Exporter en PDF / Imprimer',
      btn_top: 'Haut de page',
      btn_view_blueprint: 'Plan technique',
      btn_view_state: 'Voir l’état actuel',
      concept_badge: 'Aperçu Living Canvas',
      d_actiond: 'Action1',
      d_actiond_r: 'Validation des propositions d\'actions, critique de politique, permis uniques',
      d_actiond_s: 'Magasin de politiques',
      d_agentd: 'Agent1',
      d_agentd_r: 'Superviseur des capsules d\'agents, cycle de vie Landlock/cgroups',
      d_agentd_s: 'Descripteurs de capsules',
      d_contextd: 'Context1',
      d_contextd_r: 'Rappel de mémoire associative, décroissance temporelle, activation de contexte',
      d_contextd_s: 'Graphe de contexte',
      d_epistemicd: 'Epistemic1',
      d_epistemicd_r: 'Revendications épistémiques, vérification des preuves, gestion des croyances',
      d_epistemicd_s: 'Magasin épistémique',
      d_eventd: 'Event1',
      d_eventd_r: 'Écrivain canonique du Journal, hachage SHA-256 en chaîne, ordre causal',
      d_eventd_s: 'SQLite v3 (<code>journal.db</code>)',
      d_healthd: 'Health1',
      d_healthd_r: 'Santé des sous-systèmes, surveillance homéostatique, graphe de déficits',
      d_healthd_s: 'Snapshots de santé',
      d_identityd: 'Identity1',
      d_identityd_r: 'Continuité de session logique, continuité du sujet et biographie',
      d_identityd_s: 'Marqueur d\'exécution & Journal',
      d_intentiond: 'Intention1',
      d_intentiond_r: 'Suivi des engagements, obligations et état terminal des intentions',
      d_intentiond_s: 'Enregistrements du Journal',
      d_lifecycled: 'Lifecycle1',
      d_lifecycled_r: 'Exécutions de maintenance adossées à des preuves, ordonnancement veille/sommeil',
      d_lifecycled_s: 'Registres de cycle de vie',
      d_meaningd: 'Meaning1',
      d_meaningd_r: 'Analyseur d\'actes cognitifs, extraction d\'intention en dialogue naturel',
      d_meaningd_s: 'Règles grammaticales',
      d_predictord: 'Predictor1',
      d_predictord_r: 'Étalonnage statistique, prédictions d\'horizon et validation de forecasts',
      d_predictord_s: 'Modèles d\'étalonnage',
      d_presenced: 'Presence1',
      d_presenced_r: 'Snapshot agrégé de présentation, contrôle des commandes, signaux UI',
      d_presenced_s: 'Cache de projections',
      d_selfd: 'Self1',
      d_selfd_r: 'Modèle de soi autobiographique, projection de capacités et auto-évaluation',
      d_selfd_s: 'État de soi dérivé',
      d_telemetryd: 'Telemetry1',
      d_telemetryd_r: 'Métriques de l\'hôte, santé des services, états des sockets et diagnostics',
      d_telemetryd_s: 'Flux de télémétrie',
      d_workspaced: 'Workspace1',
      d_workspaced_r: 'Attention temporaire délimitée, cartes actives, contexte focal',
      d_workspaced_s: 'Anneau d\'attention en mémoire',
      design_sub: 'Un canevas 2D infini où cartes, ensembles, relations et télémétrie système coexistent sans chaos de fenêtres superposées.',
      desktop_stable: 'Substrat Développeur',
      desktop_sub: 'Un environnement cognitif souverain. Conçu pour les ingénieurs.',
      desktop_welcome: 'Bienvenue sur CYBOU',
      evidence_source: 'Dépôt Source',
      evidence_source_desc: 'Consultez le code source, les ADRs et l\'historique de développement sur GitHub',
      evidence_state: 'État Actuel',
      evidence_state_desc: 'Architecture implémentée, capacités actives et limites connues',
      evidence_tests: 'Preuves de Test',
      evidence_tests_desc: '88+ suites de tests validées avec 100% de succès',
      exp_h2_1: 'Observation continue.',
      exp_h2_2: 'Ancrée dans des preuves cryptographiques.',
      exp_lead: 'Charge CPU, pression mémoire, latence I/O, descripteurs ouverts et unités systemd sont continuellement enregistrés dans un journal SHA-256 inviolable. Les conclusions reposent sur des mesures télémétriques vérifiables plutôt que sur des affirmations probabilistes de LLM.',
      faq_a1: 'Cybou est un environnement opérationnel orienté agents et un bureau spatial open-source en cours de développement actif, conçu en 100% Rust pour Debian 13 Linux. Un plan de contrôle local déterministe nommé Mind observe et gouverne en continu la machine hôte, tandis que les capsules d\'agents autonomes et les modèles restent strictement confinés et remplaçables.',
      faq_a2: 'Non, Cybou est actuellement en phase de pré-version / aperçu développeur. Le plan de contrôle Mind, le registre cryptographique, le bureau Living Canvas et les capsules d\'agents sont pleinement fonctionnels en environnement de développement et de laboratoire. Il s\'adresse aux développeurs, chercheurs et bâtisseurs d\'infrastructures souveraines.',
      faq_a3: 'Un modèle n\'est pas Mind. Un agent n\'est pas Mind. L\'accès à un outil n\'est pas une permission. Les assistants IA traditionnels regroupent mémoire, identité, exécution et sécurité dans un seul prompt probabiliste. Cybou instaure une architecture zéro-confiance : mémoire, identité, santé et cycle de vie sont des micro-démons D-Bus séparés. Les agents IA tournent dans des capsules sécurisées par le noyau (cgroups v2, Landlock LSM) et ne peuvent que proposer des actions soumises à validation.',
      faq_a4: 'Non. Cybou est autosuffisant en local par conception : la cognition, l\'identité, la biographie, l\'analyse du sens et le bureau Living Canvas ne requièrent aucune connexion cloud, aucun GPU dédié et aucune clé API externe. Il ne collecte et ne transmet aucune télémétrie distante.',
      faq_a5: 'Living Canvas est une interface spatiale haute performance conçue entièrement en Leptos et WebAssembly (Rust). Elle communique via une passerelle Axum sécurisée en JSON typé et flux d\'événements temps réel, proposant 20+ cartes spécialisées dont un terminal sandboxé, un gestionnaire de fichiers hôte, un graphe cognitif et la télémétrie système.',
      faq_a6: 'Cybou applique un pipeline rigoureux à deux phases (Action1 / Executor1) : les agents ou utilisateurs proposent des actions typées ; Mind évalue la politique de sécurité et délivre un permis à usage unique ; cybou-executord réalise l\'effet physique ; et la télémétrie réobserve de façon indépendante le résultat. Si la télémétrie ne confirme pas le succès, l\'action est marquée échouée dans le Journal cryptographique.',
      faq_a7: 'Le code et la documentation de Cybou sont sous licence MIT. Les ressources graphiques originales sont sous licence Creative Commons CC BY-SA 4.0. Le dépôt est 100% conforme à la spécification REUSE.',
      faq_h2_1: 'Questions',
      faq_h2_2: 'fréquentes.',
      faq_q1: 'Qu\'est-ce que Cybou ?',
      faq_q2: 'Cybou est-il prêt pour la production ?',
      faq_q3: 'En quoi est-ce différent d\'ajouter un chatbot IA à un OS ?',
      faq_q4: 'Cybou nécessite-t-il des services cloud ou envoie-t-il de la télémétrie ?',
      faq_q5: 'Avec quoi l\'interface Living Canvas est-elle construite ?',
      faq_q6: 'Comment les actions et l\'exécution des agents sont-elles gouvernées ?',
      faq_q7: 'Quelles licences open-source s\'appliquent ?',
      feat1_desc: 'Interface spatiale accélérée par GPU en Leptos/WASM offrant 20+ cartes spécialisées (Terminal, Fichiers, Graphe, Moniteur, Notes) avec une réactivité sub-milliseconde sur un canevas 2D infini.',
      feat1_title: 'Bureau Spatial Living Canvas',
      feat2_desc: 'Les agents IA s\'exécutent dans des bacs à sable non privilégiés délimités par cgroups v2, Landlock LSM, des espaces de noms de montage et un egress réseau médié.',
      feat2_title: 'Capsules d\'Agents Isolées par le Noyau',
      feat3_desc: 'Journal SQLite v3 en ajout seul avec chaînage de hachages SHA-256, ordre causal et effacement cryptographique transitif robuste aux pannes (ADR-0028).',
      feat3_title: 'Registre Cryptographique Event1',
      footer_sub: 'Environnement Souverain Orienté Agents · 100% Rust & WebAssembly · Debian 13.',
      found_h2_1: 'Proposition Typée.',
      found_h2_2: 'Exécution Gouvernée.',
      found_lead: 'Cybou découple strictement la proposition de l\'exécution. Les agents et utilisateurs formulent des propositions d\'actions typées. Mind les évalue selon la politique de sécurité et les preuves empiriques, émettant un permis à usage unique pour Executor1. Les effets réels sont réobservés de manière indépendante par la télémétrie.',
      gen145_desc: 'Registre cryptographique · 46k+ événements',
      gen146_desc: 'Sandboxing noyau & capsules · actif',
      gen147_desc: 'Actif · Plan de contrôle Mind vérifié',
      gen_active: 'Actif & Vérifié',
      gen_aug2: 'En direct',
      gen_console_title: 'Télémétrie du substrat',
      gen_status: 'Substrat Mind actif & vérifié',
      gen_yesterday: 'Vérifié',
      hero_btn_blueprint: 'Lire le plan technique',
      hero_btn_explore: 'Explorer le code & l\'architecture',
      hero_eyebrow: 'Aperçu développeur · 100% Rust & WebAssembly · Debian 13',
      hero_h1_1: 'L\'Environnement Opérationnel',
      hero_h1_2: 'Souverain et Orienté Agents.',
      hero_lead: 'Cybou est un environnement opérationnel orienté agents et un bureau spatial open-source en cours de développement actif. Conçu entièrement en Rust pour Debian 13, il établit un plan de contrôle cognitif local déterministe (Mind), des capsules d\'agents isolées par le noyau, une gouvernance d\'actions à deux phases et un journal cryptographique inviolable. 100% autosuffisant en local avec zéro dépendance au cloud.',
      kicker_design: '04 · Modèle Spatial',
      kicker_faq: '07 · FAQ & Architecture',
      kicker_foundation: '02 · Gouvernance',
      kicker_interface: '01 · Interface & Substrat',
      kicker_principles: '05 · Invariants Fondamentaux',
      kicker_progress: '03 · Progression Technique',
      kicker_roadmap: '06 · Feuille de Route',
      label_palette: 'Thème',
      label_symbol: 'Primitive Spatiale',
      label_typography: 'Performance',
      launcher_search: 'Rechercher des cartes spatiales & démons',
      mark_desc: 'Les cartes et regroupements s\'alignent sur des guides dynamiques avec zoom instantané et agencement multi-modes.',
      metric_contrast: 'Espace de travail en pur Rust avec garanties strictes de sécurité et zéro composant simulé',
      metric_gate_a: '14 micro-démons Mind isolés avec interfaces D-Bus typées',
      metric_tasks: 'Portes automatisées CI/CD : formatage, clippy, tests unitaires et validation KVM en direct',
      metric_v_crates: '35 Crates Rust',
      metric_v_services: '14 Démons Mind',
      metric_v_tests: '88+ Suites de Tests (100% Réussite)',
      nav_blueprint: 'Plan technique',
      nav_design: 'Canevas Spatial',
      nav_experience: 'Interface',
      nav_faq: 'FAQ',
      nav_foundation: 'Fondation',
      nav_github: 'GitHub',
      nav_partners: 'Partenaires',
      nav_partners_footer: 'Partenaires & Parrainage',
      nav_progress: 'Progression',
      nav_roadmap: 'Feuille de route',
      p1_head: 'Proposition Typée',
      p1_text: 'Opérations conformes à un protocole fermé vérifié, jamais de commandes shell brutes.',
      p2_head: 'Évaluation de Politique',
      p2_text: 'Vérification par rapport aux constats empiriques, aux règles de sécurité et aux budgets de ressources.',
      p3_head: 'Résultat Indépendant',
      p3_text: 'Confirmation physique du résultat par réobservation télémétrique avant tout engagement dans l\'historique.',
      palette_name: 'Minéral Sombre · Menthe Aurore',
      part_badge: 'Partenariats & Soutien',
      part_contact_head: 'Contact officiel',
      part_contact_text: 'Pour toute demande d’entreprise, presse, divulgation de sécurité, collaboration avec les distributions Linux ou support matériel :',
      part_crypto_head: 'Dons en cryptomonnaie',
      part_crypto_text: 'Soutenez directement Cybou en cryptomonnaie :',
      part_donate_head: 'Soutien et dons',
      part_donate_text: 'Cybou est un projet open source indépendant. Votre soutien finance les builds reproductibles, le bureau Living Canvas en Rust/WebAssembly, l’ingénierie du plan de contrôle Mind et les tests de résilience sur Debian 13.',
      part_lead: 'Contactez l’équipe Cybou, explorez des partenariats matériels et logiciels, ou soutenez le développement open source indépendant de Cybou.',
      part_title: 'Partenaires, Contact et Dons',
      pr1_desc: 'Les modèles IA sont des facultés statistiques remplaçables. La mémoire, l\'identité et les invariants système restent locaux et persistants.',
      pr1_head: 'Modèle ≠ Mind',
      pr2_desc: 'Les agents s\'exécutent dans des bacs à sable cgroups v2 et Landlock avec des plafonds stricts de ressources.',
      pr2_head: 'Autonomie Délimitée par le Noyau',
      pr3_desc: 'Zéro télémétrie distante, zéro traçage et fonctionnement 100% autonome hors ligne.',
      pr3_head: 'Privé par Construction',
      pr4_desc: 'Toute croyance cognitive est adossée à une télémétrie vérifiable et à des preuves dans le registre cryptographique.',
      pr4_head: 'Vérité contre Hallucination',
      princ_h2_1: 'Remplaçable quand possible.',
      princ_h2_2: 'Gouverné quand requis.',
      prog_h2_1: 'Ingénierie Rigoureuse.',
      prog_h2_2: 'Pré-version Open-Source.',
      prog_lead: 'La plateforme intègre 14 micro-démons D-Bus isolés, la séparation des privilèges PAM, des sockets de fichiers unprivilégiés par UID, des shells sandboxés et un stockage souverain persistant. Validée par 88+ suites de tests automatisées.',
      rm1_sub: '14 démons D-Bus · Historique durable · Épistémique · Effacement transitif · Continuité d\'identité',
      rm1_title: 'Substrat Cognitif Mind',
      rm2_sub: 'Leptos/WASM · Canevas spatial infini · 20+ cartes spécialisées · Agencement réactif',
      rm2_title: 'Interface Spatiale Living Canvas',
      rm3_sub: 'Telemetry1 · Graphe cognitif · Analyseur de dialogue naturel · Constats épistémiques',
      rm3_title: 'Observation & Signification du Dialogue',
      rm4_sub: 'Capsules Bubblewrap/Landlock · Pipeline Action1/Executor1 · Persistance d\'apprentissage continu',
      rm4_title: 'Capsules d\'Agents & Stockage Souverain',
      rm_complete: 'Implémenté & Testé',
      rm_inprogress: 'En Cours de Développement',
      road_h2_1: 'Fondation souveraine.',
      road_h2_2: 'Développement actif.',
      row_body_bound: 'Télémétrie bornée sur /proc et statvfs Linux, génération d’hypothèses et statistiques robustes',
      row_body_cap: 'Observation du Corps & Télémétrie',
      row_body_ev: 'Observation en direct de plus de 46 300 contributions cryptographiques sur Debian 13',
      row_m0_bound: 'Espace Cargo verrouillé, formatage strict, clippy, métadonnées et licences',
      row_m0_cap: 'Base Rust reproductible',
      row_m0_ev: 'Portes CI automatisées sur l’ensemble des 35 crates',
      row_m14_bound: 'Rédacteur unique Event1, services D-Bus isolés et démon de projection Presence1',
      row_m14_cap: 'Mémoire acceptée & Démons isolés',
      row_m14_ev: 'Tests de bus de session D-Bus, ordre des événements et validation VM',
      row_m5_bound: 'État persistant, Lifecycle1, effets déterministes et reprise après redémarrage',
      row_m5_cap: 'Continuité & Cycle de vie de consolidation',
      row_m5_ev: 'Continuité du cycle de vie face aux redémarrages réels du serveur',
      row_m6_bound: 'Graphe de dépendances, Health1, planification homéostatique et contrats UI dégradés',
      row_m6_cap: 'Santé, Homéostasie & Récupération',
      row_m6_ev: 'Frontière de récupération et matrices d’injection de pannes',
      row_m7_bound: 'Perception, épistémique, effacement transitif Journal v3, sensibilité et mémoire associative',
      row_m7_cap: 'Cognition ancrée & Vérité épistémique',
      row_m7_ev: 'Budgets de mise à l’échelle, filtres de divulgation et vérification des hachages',
      row_m8_bound: 'Courtier de modèles remplaçable avec contrats typés et paquets d\'agents ACP',
      row_m8_cap: 'Courtage de Modèles & Intégration ACP',
      row_m8_ev: 'Intégration du pack OpenCode ACP et baux de modèles sans fuite de clés validés',
      row_m913_bound: 'Capsules Landlock/cgroups, gouvernance Action1/Executor1 et persistance d\'apprentissage continu',
      row_m913_cap: 'Capsules d\'Agents, Action1 & Persistance d\'Apprentissage',
      row_m913_ev: 'Lancement de capsules d\'agents et portes d\'induction empirique vérifiés en direct',
      row_p67_bound: 'Analyse déterministe du dialogue, génération de ResponsePlan et délais RPC bornés',
      row_p67_cap: 'Présence & Frontière du sens',
      row_p67_ev: 'Tests complets du sens et validation des actes cognitifs déterministes',
      row_w01_bound: 'Passerelle web Axum avec authentification PAM, socket non privilégié pour les fichiers et shell isolé',
      row_w01_cap: 'Living Canvas & Passerelle Web',
      row_w01_ev: 'Tests natifs, build WASM avec 0 avertissement et validation sur navigateur',
      row_w2_bound: 'Canevas spatial infini avec Shell, Fichiers, Graphe Cognitif, Moniteur, Notes, Calendrier',
      row_w2_cap: 'Bureau Spatial Living Canvas (20+ Cartes)',
      row_w2_ev: 'Vérifié sur VPS Debian 13 avec réactivité sub-milliseconde',
      skip_link: 'Passer au contenu',
      status_development: 'Substrat Actif',
      status_runtime: 'Base Vérifiée',
      status_verified: 'Opérationnel',
      tag_ai: 'Vérité épistémique, zéro hallucination',
      tag_built_with: '100% Rust · WebAssembly',
      tag_done: 'FAIT',
      tag_inprogress: 'ACTIF',
      tag_planned: 'PRÉVU',
      tag_telemetry: 'Zéro télémétrie distante',
      tag_unplugged: '100% Autosuffisant en local',
      th_boundary: 'Frontière d\'Architecture',
      th_bus: 'Interface D-Bus',
      th_capability: 'Capacité & Périmètre',
      th_daemon: 'Processus Démon',
      th_evidence: 'Preuve de Vérification',
      th_milestone: 'Jalon',
      th_responsibility: 'Responsabilité',
      th_state: 'État Persistant',
      th_status: 'Statut',
      toc_architecture: 'Architecture',
      toc_faq: 'FAQ',
      toc_implementation: 'Implémentation',
      toc_safety: 'Frontières de sécurité',
      toc_security: 'Sécurité & licences',
      toc_vision: 'Vision',
      type_desc: 'Rendu WebAssembly ultra-rapide garantissant une navigation spatiale fluide et sans distraction.',
      type_head: 'Zéro Avertissement · Leptos WASM',
      visual_caption: 'Environnement spatial Living Canvas — canevas infini avec 20+ cartes interactives',
      ws_active: 'Espace spatial {n} actif',
    },
    ru: {
      app_code: 'Капсула агента',
      app_files: 'Файлы 2.0',
      app_ready: 'Карточка {app} активна на холсте',
      app_settings: 'Настройки',
      app_web: 'Когнитивный граф',
      bp_ai_point1: '<strong>Владение важнее интеллекта:</strong> Модель ≠ Личность, UI ≠ Mind, Внимание ≠ Биография, и Предложение ≠ Авторизация.',
      bp_ai_point2: '<strong>Надёжность важнее видимости:</strong> Состояние проецируется только после фиксации в криптографическом реестре Event1; консолидация никогда не переписывает историю.',
      bp_ai_point3: '<strong>Ограниченная деградация:</strong> Health1 публикует типизированные дефициты и прогресс восстановления; составные операции делят один монотонный дедлайн.',
      bp_ai_point4: '<strong>Изоляция ядром:</strong> Автономные агенты исполняются в Landlock-капсулах с лимитами cgroups; действия требуют типизированных пермитов и независимой проверки.',
      bp_badge: 'Технический Blueprint · Суверенная ИИ-архитектура',
      bp_banner_btn: 'Изучить архитектуру',
      bp_banner_desc: 'Техническая архитектура подробно описывает 14 демонов Mind, изолированные ядром капсулы агентов, пространственный десктоп Leptos/WASM, контур Action1/Executor1 и криптографический реестр Event1.',
      bp_banner_h2_1: 'Агентная операционная среда,',
      bp_banner_h2_2: 'построенная слой за слоем на принципах доверия.',
      bp_banner_kicker: 'Суверенная архитектура',
      bp_layer1_desc: 'Пакеты Debian 13, cgroups v2, Landlock LSM и пользовательские службы systemd, развёрнутые с явными гейтами сборки и восстановления. Debian — авторитетная среда сборки и продакшена.',
      bp_layer1_title: 'Слой 1: Воспроизводимое тело Linux',
      bp_layer2_desc: '14 изолированных пользовательских демонов systemd, взаимодействующих по типизированным контрактам D-Bus. Event1 — единственный писатель журнала; Presence1 проецирует состояние; Meaning1 детерминированно разбирает диалог.',
      bp_layer2_title: 'Слой 2: Детерминированный рантайм Mind',
      bp_layer3_desc: 'Фронтенд на чистом Rust/WebAssembly (Leptos), работающий через аутентифицированный шлюз Axum. Предоставляет 20+ специализированных карточек с субмиллисекундным откликом и 0 ворнингов.',
      bp_layer3_title: 'Слой 3: Пространственный десктоп Living Canvas',
      bp_lead: 'Техническая спецификация управляющего контура Mind, изолированных капсул агентов, десктопа Living Canvas на Leptos/WASM и криптографического реестра Event1 на Debian 13.',
      bp_lic1: '<strong>Лицензия кода:</strong> MIT для всех пакетов Rust, демонов, шлюза и скриптов.',
      bp_lic2: '<strong>Визуальные материалы:</strong> Creative Commons Attribution-ShareAlike 4.0 для логотипов, обоев и тем.',
      bp_lic3: '<strong>Границы доверия:</strong> Строгая PAM-аутентификация через cybou-authd; непривилегированный доступ к файлам через сокеты UID.',
      bp_lic4: '<strong>Граница интерфейса:</strong> Веб-шлюз применяет строгие заголовки CSP, изолированные сессии и политику no-store.',
      bp_lic5: '<strong>Приватность и стирание:</strong> Конверты несут оси чувствительности; транзитивное криптографическое стирание безопасно уничтожает данные по требованию.',
      bp_lic6: '<strong>Ноль облачных зависимостей:</strong> Работает 100% локально без обязательного облачного аккаунта, API-ключей и удалённых моделей.',
      bp_lic7: '<strong>Документы по безопасности:</strong> Модели угроз, границы доверия и правила изоляции ядра под версионным контролем.',
      bp_meta_foundation: 'Основа:',
      bp_meta_foundation_v: 'Debian 13, воркспейс 100% Rust, cgroups v2, Landlock LSM',
      bp_meta_interface: 'Интерфейс:',
      bp_meta_interface_v: 'Living Canvas (Rust / Leptos / WebAssembly)',
      bp_meta_status: 'Статус:',
      bp_meta_status_v: 'Mind, Капсулы, Шелл, Файлы, Обучение и Хранилище работают в проде',
      bp_meta_target: 'Целевая платформа: Debian 13 (Trixie)',
      bp_meta_updated: 'Обновлено:',
      bp_meta_updated_v: 'Август 2026',
      bp_meta_version: 'Pre-Release 0.1 · 100% Rust',
      bp_s1_card1_text: '14 независимых D-Bus микро-демонов (<code>org.cybou.Mind.*</code>), управляющих идентичностью, биографией, намерениями, контекстом и здоровьем без вероятностных галлюцинаций.',
      bp_s1_card1_title: 'Детерминированный контур Mind',
      bp_s1_card2_text: 'Капсулы изолированы через cgroups v2, Landlock LSM и приватные пространства имён. Агенты предлагают типизированные действия вместо бесконтрольного запуска шелла.',
      bp_s1_card2_title: 'Капсулы агентов, изолированные ядром',
      bp_s1_card3_text: 'Интерфейс на Leptos/WASM с аппаратным ускорением, предоставляющий 20+ карточек (терминал, файлы, граф, мониторинг, заметки) через локальный шлюз Axum.',
      bp_s1_card3_title: 'Пространственный десктоп Living Canvas',
      bp_s1_card4_text: 'Двухфазный пайплайн: Предложение действия &rarr; Пропуск безопасности &rarr; Executor1 &rarr; Независимая перепроверка результата телеметрией.',
      bp_s1_card4_title: 'Контур Action1 & Executor1',
      bp_s3_p1: '<strong>Чистый Rust/WASM:</strong> Никакого ручного JavaScript; все 20+ карточек компилируются в единый бинарник WebAssembly без ворнингов.',
      bp_s3_p2: '<strong>Аутентифицированный веб-шлюз:</strong> <code>cybou-web-gateway</code> связывает сессии браузера с D-Bus демонами через JSON-RPC и SSE-потоки.',
      bp_s3_p3: '<strong>Двойной движок Shell:</strong> Поддерживает безопасный демонстрационный шелл (<code>cybou-jailfs</code>) и интерактивный PTY-терминал хоста (<code>cybou-shelld</code>).',
      bp_s3_p4: '<strong>Файловая система хоста:</strong> Многопанельный файловый менеджер на базе <code>cybou-host-filesd</code> через непривилегированные сокеты UID.',
      bp_s4_p1: '<strong>Капсулы агентов:</strong> Сторонние агенты (например, OpenCode ACP) запускаются в песочницах Bubblewrap/Landlock с приватными пространствами имён и лимитами CPU/RAM.',
      bp_s4_p2: '<strong>Брокеридж моделей:</strong> Модели арендуются через локальный брокер с контролем чувствительности и лимитов; ключи API никогда не попадают внутрь капсул.',
      bp_s4_p3: '<strong>Двухфазный контур:</strong> Предложение действия &rarr; Оценка политик &rarr; Одноразовый пропуск &rarr; Executor1 &rarr; Телеметрическая перепроверка.',
      bp_s4_p4: '<strong>Криптографический журнал:</strong> Каждое наблюдение и результат подписываются и вносятся в неизменяемый реестр с SHA-256 хэш-цепочками.',
      bp_sec1_lead: 'CYBOU — это агентно-ориентированная операционная среда и когнитивный контур управления, написанный полностью на Rust для Debian 13. Он непрерывно наблюдает за собственным состоянием, диагностирует аномалии и выполняет контролируемые действия через капсулы агентов и криптографический журнал.',
      bp_sec1_p1: 'CYBOU — это суверенная, спроектированная под агентов операционная среда, созданная на 100% Rust. Она формирует детерминированный слой когнитивного контроля (Mind), владеющий биографией, идентичностью, обязательствами, предсказаниями, вниманием, здоровьем и восстановлением, в связке с непривилегированным десктопом Living Canvas. Модели и агенты работают как изолированные гости внутри капсул ядра.',
      bp_sec1_title: '1. Архитектурный обзор',
      bp_sec2_lead: 'Mind работает как 14 изолированных пользовательских служб systemd, взаимодействующих по сессионной шине D-Bus:',
      bp_sec2_p1: 'CYBOU проводит строгое разделение между Телом (Body), Разумом (Mind) и Присутствием (Presence). Долговременное состояние принадлежит явным процессам-владельцам; интерфейсы являются чистыми проекциями.',
      bp_sec2_title: '2. Топология процессов и архитектура демонов',
      bp_sec3_lead: 'Living Canvas предоставляет бесконечное 2D-рабочее пространство, созданное полностью на Leptos и WebAssembly:',
      bp_sec3_p1: 'Когнитивный субстрат обеспечивает полную проверяемость и безопасность, поддерживая фундаментальные инварианты во всех подсистемах:',
      bp_sec3_title: '3. Архитектура пространственного десктопа Living Canvas',
      bp_sec4_lead: 'Модель безопасности CYBOU гарантирует, что автономные агенты не могут выполнить непроверенные действия:',
      bp_sec4_p1: 'Возможности платформы непрерывно верифицируются через 88+ автоматических тестовых сьютов и боевые развёртывания на серверах Debian 13.',
      bp_sec4_title: '4. Управление, капсулы агентов и пайплайн Action1',
      bp_sec5_lead: 'Вехи платформы, проверенные в среде Debian 13 Linux:',
      bp_sec5_p1: 'CYBOU — свободное программное обеспечение, созданное для суверенитета, приватности и независимости от облаков:',
      bp_sec5_title: '5. Инженерные доказательства и вехи реализации',
      bp_sec6_title: '6. Часто задаваемые вопросы',
      bp_sources_lead: 'Этот веб-документ является сводкой. Точные утверждения о реализации зафиксированы в репозитории:',
      bp_sources_title: 'Канонические источники и верификация',
      bp_src1: '— авторитетная реализованная граница и известные ограничения.',
      bp_src2: '— топология процессов, владение данными и когнитивные инварианты.',
      bp_src3: '— доказательства тестов, инъекции сбоев и KVM live-bus проверка.',
      bp_src4: '— подробный журнал реализации и текущие активные вехи.',
      bp_stack1: '<strong>Базовая система:</strong> Debian 13 Linux (trixie) с пользовательским менеджером systemd, cgroups v2, Landlock LSM и изоляцией bubblewrap.',
      bp_stack10: '<strong>Ноль облачной привязки:</strong> Полностью локально-достаточная работа без обязательных облачных аккаунтов, API-ключей и телеметрии.',
      bp_stack2: '<strong>Воркспейс Rust:</strong> 35 модульных пакетов: протоколы, криптографическое хранение, fabric D-Bus, рантайм агентов, веб-шлюз и десктоп WASM Living Canvas.',
      bp_stack3: '<strong>Веб-шлюз:</strong> Высокопроизводительный шлюз Axum HTTP/WebSocket с аутентификацией PAM, непривилегированным сокетом файлов хоста и потоком событий.',
      bp_stack4: '<strong>Капсулы агентов:</strong> Изолированные среды выполнения с декларативными профилями оператора, лимитами расходов, бюджетами CPU/RAM и брокерируемым трафиком.',
      bp_stack5: '<strong>Living Canvas:</strong> Пространственный бесконечный десктоп на Leptos/WASM с 20+ реактивными карточками, магнитными направляющими и субмиллисекундным откликом.',
      bp_stack6: '<strong>Криптографический реестр:</strong> Append-only журнал SQLite v3 с хэш-цепочками SHA-256, причинно-следственной историей и криптографическим стиранием.',
      bp_stack7: '<strong>Слой управления Mind:</strong> 14 изолированных D-Bus микро-демонов, отвечающих за эпистемику, смысл, ассоциативный контекст, предсказания, здоровье и жизненный цикл.',
      bp_stack8: '<strong>Двухфазное управление:</strong> Типизированные предложения действий оцениваются политиками, выдавая одноразовые пермиты для Executor1.',
      bp_stack9: '<strong>Суверенная персистентность:</strong> Атомарное локальное хранение заметок, контактов, событий календаря и линий артефактов непрерывного обучения.',
      bp_stack_title: 'Базовый системный стек',
      bp_title: 'Техническая Архитектура CYBOU',
      bp_topology_title: 'Топология процессов и владения',
      btn_back_home: '← На главную',
      btn_print_pdf: 'Экспорт в PDF / Печать',
      btn_top: 'Наверх',
      btn_view_blueprint: 'Архитектура',
      btn_view_state: 'Смотреть текущее состояние',
      concept_badge: 'Превью Living Canvas',
      d_actiond: 'Action1',
      d_actiond_r: 'Валидация предложений, критика политик, одноразовые разрешения',
      d_actiond_s: 'Хранилище политик',
      d_agentd: 'Agent1',
      d_agentd_r: 'Супервизор капсул агентов, управление Landlock/cgroups',
      d_agentd_s: 'Дескрипторы капсул',
      d_contextd: 'Context1',
      d_contextd_r: 'Ассоциативная память, затухание во времени, контекстная активация',
      d_contextd_s: 'Граф контекста',
      d_epistemicd: 'Epistemic1',
      d_epistemicd_r: 'Эпистемические утверждения, верификация фактов, управление знаниями',
      d_epistemicd_s: 'Хранилище знаний',
      d_eventd: 'Event1',
      d_eventd_r: 'Каноническая запись в Журнал, SHA-256 цепочки хэшей, причинный порядок',
      d_eventd_s: 'SQLite v3 (<code>journal.db</code>)',
      d_healthd: 'Health1',
      d_healthd_r: 'Здоровье подсистем, гомеостаз, граф дефицитов возможностей',
      d_healthd_s: 'Снимки здоровья',
      d_identityd: 'Identity1',
      d_identityd_r: 'Непрерывность логической сессии, личность и история жизни',
      d_identityd_s: 'Рантайм-маркер и Журнал',
      d_intentiond: 'Intention1',
      d_intentiond_r: 'Учёт обязательств, целей и терминальное состояние намерений',
      d_intentiond_s: 'Записи Журнала',
      d_lifecycled: 'Lifecycle1',
      d_lifecycled_r: 'Сессии консолидации и сна/бодрствования на основе доказательств',
      d_lifecycled_s: 'Записи циклов',
      d_meaningd: 'Meaning1',
      d_meaningd_r: 'Парсер когнитивных актов, извлечение намерений из диалога',
      d_meaningd_s: 'Правила грамматики',
      d_predictord: 'Predictor1',
      d_predictord_r: 'Статистическая калибровка, горизонты прогнозирования и валидация',
      d_predictord_s: 'Модели калибровки',
      d_presenced: 'Presence1',
      d_presenced_r: 'Агрегированный снимок состояния, фильтрация команд, сигналы UI',
      d_presenced_s: 'Кэш проекций',
      d_selfd: 'Self1',
      d_selfd_r: 'Автобиографическая модель себя, проекция возможностей и самооценка',
      d_selfd_s: 'Производное состояние',
      d_telemetryd: 'Telemetry1',
      d_telemetryd_r: 'Метрики хоста, состояние сервисов, сокетов и диагностика',
      d_telemetryd_s: 'Потоки телеметрии',
      d_workspaced: 'Workspace1',
      d_workspaced_r: 'Ограниченное внимание, активные карточки, текущий контекст',
      d_workspaced_s: 'Кольцевой буфер в памяти',
      design_sub: 'Бесконечная 2D-плоскость, где карточки, колоды, связи и телеметрия сосуществуют без хаоса перекрывающихся окон.',
      desktop_stable: 'Developer Substrate',
      desktop_sub: 'Суверенная когнитивная среда. Сделано инженерами для инженеров.',
      desktop_welcome: 'Добро пожаловать в CYBOU',
      evidence_source: 'Исходный код',
      evidence_source_desc: 'Исходный код, архитектурные решения (ADR) и история разработки на GitHub',
      evidence_state: 'Текущее состояние',
      evidence_state_desc: 'Реализованная архитектура, активные возможности и известные ограничения',
      evidence_tests: 'Доказательства тестов',
      evidence_tests_desc: '88+ сьютов тестов с прохождением на 100%',
      exp_h2_1: 'Непрерывное наблюдение.',
      exp_h2_2: 'Подтверждено криптографическими уликами.',
      exp_lead: 'Нагрузка CPU, память, задержка ввода-вывода, открытые дескрипторы и службы systemd непрерывно записываются в неизменяемый журнал на SHA-256 хэш-цепочках. Любые выводы подкреплены проверяемыми цифрами телеметрии, а не вероятностными утверждениями языковых моделей.',
      faq_a1: 'CYBOU — это открытая суверенная агентно-ориентированная операционная среда и пространственный рабочий стол в активной разработке, написанная на 100% чистом Rust для Debian 13 Linux. Детерминированный локальный контур управления Mind непрерывно наблюдает и управляет хост-машиной, а автономные капсулы агентов, модели и инструменты остаются строго изолированными и заменяемыми.',
      faq_a2: 'Нет, сейчас CYBOU находится в активной стадии предварительной разработки (Developer Preview / Research Substrate). Ядро Mind, криптографический реестр, десктоп Living Canvas и изолированные капсулы агентов полностью функционируют в среде разработки и лабораторных тестах. Система ориентирована на разработчиков, исследователей и создателей суверенной инфраструктуры.',
      faq_a3: 'Модель — это не Mind. Агент — это не Mind. Доступ к утилитам — это не право на действие. Обычные AI-помощники смешивают память, идентичность, выполнение команд и безопасность в одном вероятностном промпте. В CYBOU реализована архитектура нулевого доверия: память, личность, здоровье и жизненный цикл разделены на независимые D-Bus микро-демоны. AI-агенты запускаются в изолированных капсулах ядра (cgroups v2, Landlock LSM) и могут лишь предлагать действия, проходящие строгий аудит политик безопасности.',
      faq_a4: 'Нет. CYBOU полностью локально-достаточен: базовое мышление, идентичность, биография, разбор смысла и рабочий стол Living Canvas не требуют подключения к облаку, внешних серверов, GPU и API-ключей. Система не собирает и никуда не отправляет телеметрию.',
      faq_a5: 'Living Canvas — это пространственный интерфейс на Leptos и WebAssembly (Rust). Он взаимодействует с защищённым локальным веб-шлюзом Axum через типизированный JSON и потоки событий реального времени. Включает 20+ карточек: изолированный шелл, файловый менеджер хоста, когнитивный граф и телеметрию.',
      faq_a6: 'CYBOU использует двухфазный контур Action1 / Executor1: агент или оператор предлагает действие; Mind валидирует политику безопасности по реальным системным уликам и выдаёт одноразовый пропуск; cybou-executord выполняет физическое действие; затем телеметрия независимо перепроверяет результат. Если успех не подтверждён, действие признаётся неудачным и фиксируется в криптографическом журнале.',
      faq_a7: 'Код и документация CYBOU лицензированы под лицензией MIT. Оригинальные графические материалы распространяются под лицензией Creative Commons CC BY-SA 4.0. Репозиторий на 100% соответствует спецификации REUSE.',
      faq_h2_1: 'Часто задаваемые',
      faq_h2_2: 'вопросы.',
      faq_q1: 'Что такое CYBOU?',
      faq_q2: 'Готова ли система к промышленному использованию (production)?',
      faq_q3: 'Чем это отличается от добавления AI-чатбота в операционную систему?',
      faq_q4: 'Требуются ли облачные сервисы или передача телеметрии?',
      faq_q5: 'На чём построен интерфейс Living Canvas?',
      faq_q6: 'Как контролируются действия агентов и выполнение команд?',
      faq_q7: 'Под какими лицензиями распространяется проект?',
      feat1_desc: 'Высокопроизводительный интерфейс на Leptos/WASM с аппаратным ускорением, предоставляющий 20+ специализированных карточек (терминал, файлы, граф, мониторинг, заметки) на бесконечном 2D-холсте.',
      feat1_title: 'Пространственный десктоп Living Canvas',
      feat2_desc: 'ИИ-агенты запускаются в непривилегированных песочницах под контролем Linux cgroups v2, Landlock LSM, частных пространств имён и управляемого сетевого доступа.',
      feat2_title: 'Капсулы агентов, изолированные ядром',
      feat3_desc: 'Журнал append-only на SQLite v3 с SHA-256 хэш-цепочками, причинно-следственным порядком и надёжным транзитивным криптографическим стиранием (ADR-0028).',
      feat3_title: 'Криптографический реестр Event1',
      footer_sub: 'Суверенная агентная среда · 100% Rust & WebAssembly · Debian 13.',
      found_h2_1: 'Типизированное предложение.',
      found_h2_2: 'Контролируемое исполнение.',
      found_lead: 'CYBOU строго разделяет предложение действия и его физическое исполнение. Агенты и пользователи формируют типизированные предложения. Mind оценивает их по правилам безопасности и уликам, выдавая одноразовый пропуск Executor1. Истинные результаты независимо перепроверяются телеметрией.',
      gen145_desc: 'Криптографический реестр · 46k+ событий',
      gen146_desc: 'Изоляция ядром и капсулы · активны',
      gen147_desc: 'Текущий · Контур управления Mind проверен',
      gen_active: 'Активен и проверен',
      gen_aug2: 'Live',
      gen_console_title: 'Телеметрия субстрата',
      gen_status: 'Субстрат Mind активен и проверен',
      gen_yesterday: 'Проверено',
      hero_btn_blueprint: 'Техническая архитектура',
      hero_btn_explore: 'Изучить код и архитектуру',
      hero_eyebrow: 'Developer Preview · 100% Rust и WebAssembly · Debian 13',
      hero_h1_1: 'Суверенная, Агентная',
      hero_h1_2: 'Когнитивная Операционная Среда.',
      hero_lead: 'CYBOU — это открытая, агентно-ориентированная операционная среда и пространственный рабочий стол в активной разработке. Построена на 100% чистом Rust для Debian 13 Linux: детерминированный локальный управляющий контур Mind, изоляция агентов ядром Linux, двухфазное исполнение действий и криптографический журнал событий. 100% локальная автономность с нулевой зависимостью от облаков.',
      hour: '2-digit',
      kicker_design: '04 · Пространственная модель',
      kicker_faq: '07 · Частые вопросы и архитектура',
      kicker_foundation: '02 · Управление и безопасность',
      kicker_interface: '01 · Интерфейс и субстрат',
      kicker_principles: '05 · Базовые инварианты',
      kicker_progress: '03 · Инженерный прогресс',
      kicker_roadmap: '06 · План и статус',
      label_palette: 'Тема',
      label_symbol: 'Пространственный примитив',
      label_typography: 'Производительность',
      launcher_search: 'Поиск пространственных карточек и демонов',
      mark_desc: 'Карточки и колоды магнитно стыкуются по направляющим с мгновенным масштабированием и сворачиванием.',
      metric_contrast: 'Чистый воркспейс Rust со строгими гарантиями безопасности и полным отсутствием симуляций',
      metric_gate_a: '14 изолированных микро-демонов Mind с типизированными D-Bus контрактами',
      metric_tasks: 'Автоматизированные гейты CI/CD: форматирование, clippy, модульные тесты и KVM-тесты live-bus',
      metric_v_crates: '35 Пакетов Rust',
      metric_v_services: '14 Демонов Mind',
      metric_v_tests: '88+ Сьютов тестов (100% Pass)',
      minute: '2-digit',
      nav_blueprint: 'Архитектура',
      nav_design: 'Холст',
      nav_experience: 'Интерфейс',
      nav_faq: 'FAQ',
      nav_foundation: 'Фундамент',
      nav_github: 'GitHub',
      nav_partners: 'Партнёры',
      nav_partners_footer: 'Партнёры и поддержка',
      nav_progress: 'Прогресс',
      nav_roadmap: 'План',
      p1_head: 'Типизированное предложение',
      p1_text: 'Операции из строгого закрытого протокола, а не произвольные неконтролируемые shell-строки.',
      p2_head: 'Проверка политик безопасности',
      p2_text: 'Сверка с эмпирическими данными системы, правилами риска и лимитами ресурсов.',
      p3_head: 'Независимый результат',
      p3_text: 'Физический эффект подтверждается повторным замером телеметрии перед фиксацией в истории.',
      palette_name: 'Минеральный тёмный · Мятная Аврора',
      part_badge: 'Партнёрство и поддержка',
      part_contact_head: 'Официальный контакт',
      part_contact_text: 'Для корпоративных запросов, прессы, сообщений об уязвимостях, сотрудничества с дистрибутивами Linux или поддержки оборудования:',
      part_crypto_head: 'Криптодонаты',
      part_crypto_text: 'Поддержать Cybou напрямую в криптовалюте:',
      part_donate_head: 'Поддержка и донаты',
      part_donate_text: 'Cybou — независимый открытый проект. Ваша поддержка идёт на воспроизводимые сборки, пространственный десктоп Living Canvas на Rust/WebAssembly, разработку когнитивного слоя Mind и тесты отказоустойчивости на Debian 13.',
      part_lead: 'Свяжитесь с командой Cybou, обсудите партнёрство по оборудованию и софту или поддержите независимую открытую разработку Cybou.',
      part_title: 'Партнёры, контакты и донаты',
      pr1_desc: 'Нейросети — это заменяемые статистические инструменты. Память, идентичность и системные инварианты сохраняются локально.',
      pr1_head: 'Модель ≠ Mind',
      pr2_desc: 'Агенты работают внутри cgroups v2, Landlock LSM и приватных пространств имён с жёсткими лимитами ресурсов.',
      pr2_head: 'Автономия в границах ядра',
      pr3_desc: 'Нулевая удалённая телеметрия, никакого внешнего трекинга и полная автономная работа оффлайн.',
      pr3_head: 'Приватно по построению',
      pr4_desc: 'Каждое когнитивное знание подтверждается проверяемыми фактами телеметрии и криптографическим журналом.',
      pr4_head: 'Правда вместо галлюцинаций',
      princ_h2_1: 'Заменяемо, где возможно.',
      princ_h2_2: 'Управляемо, где необходимо.',
      prog_h2_1: 'Строгая инженерия.',
      prog_h2_2: 'Открытая пре-релизная разработка.',
      prog_lead: 'Платформа объединяет 14 изолированных D-Bus микро-демонов, PAM-аутентификацию, непривилегированные сокеты файлов по UID, песочницы терминала и персистентное локальное хранилище. 88+ сьютов тестов проходят со 100% успехом.',
      rm1_sub: '14 D-Bus демонов · Долговременная история · Эпистемика · Транзитивное стирание · Непрерывность идентичности',
      rm1_title: 'Когнитивный субстрат Mind',
      rm2_sub: 'Leptos/WASM · Бесконечный холст · 20+ специализированных карточек · Адаптивная вёрстка',
      rm2_title: 'Пространственный UI Living Canvas',
      rm3_sub: 'Telemetry1 · Когнитивный граф · Парсер естественного диалога · Эпистемические находки',
      rm3_title: 'Наблюдение и смысл диалога',
      rm4_sub: 'Капсулы Bubblewrap/Landlock · Контур Action1/Executor1 · Персистентность непрерывного обучения',
      rm4_title: 'Капсулы агентов и суверенное хранение',
      rm_complete: 'Реализовано и проверено',
      rm_inprogress: 'В активной разработке',
      road_h2_1: 'Суверенный фундамент.',
      road_h2_2: 'Активная разработка.',
      row_body_bound: 'Ограниченная телеметрия поверх Linux /proc и statvfs, генерация гипотез и робастная статистика',
      row_body_cap: 'Наблюдение за телом и телеметрия',
      row_body_ev: 'Живое наблюдение за 46 300+ криптографическими записями событий на Debian 13',
      row_m0_bound: 'Зафиксированный Cargo воркспейс, строгий формат, clippy, метаданные и лицензии',
      row_m0_cap: 'Воспроизводимая база на Rust',
      row_m0_ev: 'Автоматические CI-гейты по всем 35 пакетам воркспейса',
      row_m14_bound: 'Единый писатель Event1, изолированные D-Bus службы и демон проекции Presence1',
      row_m14_cap: 'Принятая память и изолированные демоны',
      row_m14_ev: 'Тесты сессионной шины D-Bus, порядок событий и валидация в VM',
      row_m5_bound: 'Персистентное состояние, Lifecycle1, детерминированные эффекты и восстановление после перезагрузки',
      row_m5_cap: 'Непрерывность и консолидация',
      row_m5_ev: 'Непрерывность жизненного цикла при реальных перезагрузках сервера',
      row_m6_bound: 'Граф зависимостей возможностей, Health1, гомеостатическое планирование и деградированные контракты UI',
      row_m6_cap: 'Здоровье, гомеостаз и восстановление',
      row_m6_ev: 'Граница восстановления и матрицы инъекции сбоев процессов',
      row_m7_bound: 'Восприятие, эпистемика, транзитивное стирание Journal v3, чувствительность и ассоциативная память',
      row_m7_cap: 'Заземлённая когниция и эпистемическая истина',
      row_m7_ev: 'Бюджеты масштабирования, фильтры раскрытия и верификация хэш-цепочек',
      row_m8_bound: 'Заменяемый брокер моделей за типизированными контрактами контекста и пакетами агентов ACP',
      row_m8_cap: 'Брокеридж моделей и интеграция ACP',
      row_m8_ev: 'Интеграция агентного пакета OpenCode ACP и аренда моделей без передачи ключей',
      row_m913_bound: 'Капсулы Landlock/cgroups, двухфазное управление Action1/Executor1 и персистентность обучения',
      row_m913_cap: 'Капсулы агентов, Action1 и обучение',
      row_m913_ev: 'Запуск капсул агентов и эмпирическая проверка гейтов обучения',
      row_p67_bound: 'Детерминированный разбор естественного диалога, генерация ResponsePlan и монотонные тайм-ауты RPC',
      row_p67_cap: 'Присутствие и граница смысла',
      row_p67_ev: 'Сквозные тесты разбора смысла и валидация когнитивных актов',
      row_w01_bound: 'Веб-шлюз Axum с PAM-аутентификацией, непривилегированным сокетом файлов хоста и изолированным шеллом',
      row_w01_cap: 'Living Canvas и веб-шлюз',
      row_w01_ev: 'Нативные тесты, сборка WASM с 0 предупреждений и браузерные гейты',
      row_w2_bound: 'Бесконечный холст: шелл, файлы, когнитивный граф, мониторинг, заметки, календарь',
      row_w2_cap: 'Пространственный десктоп Living Canvas (20+ карточек)',
      row_w2_ev: 'Проверено на VPS Debian 13 с миллисекундным временем отклика',
      skip_link: 'Перейти к содержимому',
      status_development: 'Активный субстрат',
      status_runtime: 'Проверенный базис',
      status_verified: 'Работоспособен',
      tag_ai: 'Эмпирическая правда, а не галлюцинации',
      tag_built_with: '100% Rust · WebAssembly',
      tag_done: 'ГОТОВО',
      tag_inprogress: 'АКТИВНО',
      tag_planned: 'ПЛАН',
      tag_telemetry: 'Нулевая внешняя телеметрия',
      tag_unplugged: '100% Локальная автономность',
      th_boundary: 'Архитектурная граница',
      th_bus: 'D-Bus интерфейс',
      th_capability: 'Возможность и охват',
      th_daemon: 'Процесс демона',
      th_evidence: 'Доказательство проверки',
      th_milestone: 'Веха',
      th_responsibility: 'Зона ответственности',
      th_state: 'Хранимое состояние',
      th_status: 'Статус',
      toc_architecture: 'Архитектура',
      toc_faq: 'FAQ',
      toc_implementation: 'Реализация',
      toc_safety: 'Границы безопасности',
      toc_security: 'Безопасность и лицензии',
      toc_vision: 'Видение',
      type_desc: 'Молниеносный рендеринг в WebAssembly без задержек и отвлекающих факторов.',
      type_head: '0 Ворнингов компилятора · Leptos WASM',
      visual_caption: 'Пространственный десктоп Living Canvas — бесконечный 2D-холст с 20+ интерактивными карточками',
      ws_active: 'Пространственный экран {n} активен',
    }
  };

  let currentLang = localStorage.getItem('cybou_lang') || 'en';

  const INLINE_MARKUP = /&lt;(\/?)(strong|em|code)&gt;/g;

  const applyText = (node, value) => {
    if (value.indexOf('<') === -1) {
      node.textContent = value;
      return;
    }
    const escaped = value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
    node.innerHTML = escaped.replace(INLINE_MARKUP, '<$1$2>');
  };

  const t = (key, replacements) => {
    const table = translations[currentLang] || translations.en;
    let value = table[key] || translations.en[key] || key;
    if (replacements) {
      Object.keys(replacements).forEach((name) => {
        value = value.replace('{' + name + '}', replacements[name]);
      });
    }
    return value;
  };

  const setLanguage = (lang) => {
    if (!translations[lang]) return;
    currentLang = lang;
    localStorage.setItem('cybou_lang', lang);

    document.querySelectorAll('[data-i18n]').forEach((node) => {
      const key = node.getAttribute('data-i18n');
      const value = translations[lang][key];
      if (value) {
        applyText(node, value);
      }
    });

    document.querySelectorAll('[data-i18n-placeholder]').forEach((node) => {
      const value = translations[lang][node.getAttribute('data-i18n-placeholder')];
      if (value) {
        node.setAttribute('placeholder', value);
      }
    });

    document.querySelectorAll('[data-lang-current]').forEach((node) => {
      node.textContent = lang.toUpperCase();
    });

    document.querySelectorAll('[data-lang]').forEach((btn) => {
      btn.classList.toggle('active', btn.getAttribute('data-lang') === lang);
    });

    document.documentElement.setAttribute('lang', lang);

    const canonical = document.querySelector('link[rel="canonical"]');
    if (canonical) {
      const base = canonical.href.split('?')[0];
      const self = lang === 'en' ? base : `${base}?lang=${lang}`;
      canonical.href = self;
      document.querySelector('meta[property="og:url"]')?.setAttribute('content', self);
    }
  };

  const urlParams = new URLSearchParams(window.location.search);
  const paramLang = urlParams.get('lang');
  const hashLang = window.location.hash.startsWith('#lang=') ? window.location.hash.slice(6) : null;
  const browserLang = (navigator.language || '').slice(0, 2).toLowerCase();

  const initialLang =
    (paramLang && translations[paramLang] && paramLang) ||
    (hashLang && translations[hashLang] && hashLang) ||
    localStorage.getItem('cybou_lang') ||
    (translations[browserLang] ? browserLang : 'en');

  setLanguage(initialLang);

  document.querySelectorAll('[data-lang-trigger]').forEach((trigger) => {
    trigger.addEventListener('click', (event) => {
      event.stopPropagation();
      const menu = trigger.nextElementSibling;
      const isOpen = menu.classList.toggle('open');
      trigger.classList.toggle('open', isOpen);
      trigger.setAttribute('aria-expanded', String(isOpen));
      menu.setAttribute('aria-hidden', String(!isOpen));
    });
  });

  document.querySelectorAll('[data-lang]').forEach((btn) => {
    btn.addEventListener('click', (event) => {
      event.stopPropagation();
      const lang = btn.getAttribute('data-lang');
      setLanguage(lang);
      document.querySelectorAll('[data-lang-menu]').forEach((m) => {
        m.classList.remove('open');
        m.setAttribute('aria-hidden', 'true');
      });
      document.querySelectorAll('[data-lang-trigger]').forEach((t) => {
        t.classList.remove('open');
        t.setAttribute('aria-expanded', 'false');
      });
    });
  });

  document.addEventListener('click', () => {
    document.querySelectorAll('[data-lang-menu]').forEach((m) => {
      m.classList.remove('open');
      m.setAttribute('aria-hidden', 'true');
    });
    document.querySelectorAll('[data-lang-trigger]').forEach((t) => {
      t.classList.remove('open');
      t.setAttribute('aria-expanded', 'false');
    });
  });

  // Interactive FAQ Accordion
  document.querySelectorAll('.faq-question').forEach((button) => {
    button.setAttribute('aria-expanded', 'false');
    button.addEventListener('click', () => {
      const item = button.closest('.faq-item');
      const open = !item?.classList.contains('open');
      document.querySelectorAll('.faq-item').forEach((el) => {
        el.classList.remove('open');
        el.querySelector('.faq-question')?.setAttribute('aria-expanded', 'false');
      });
      item?.classList.toggle('open', open);
      button.setAttribute('aria-expanded', String(open));
    });
  });

  const updateHeader = () => header?.classList.toggle('is-scrolled', window.scrollY > 24);
  updateHeader();
  window.addEventListener('scroll', updateHeader, { passive: true });

  menuButton?.addEventListener('click', () => {
    const open = !mobileNav.classList.contains('open');
    mobileNav.classList.toggle('open', open);
    document.body.classList.toggle('menu-open', open);
    menuButton.setAttribute('aria-expanded', String(open));
  });

  mobileNav?.querySelectorAll('a').forEach((link) => {
    link.addEventListener('click', () => {
      mobileNav.classList.remove('open');
      document.body.classList.remove('menu-open');
      menuButton?.setAttribute('aria-expanded', 'false');
    });
  });

  const setLauncher = (open) => {
    launcher?.classList.toggle('open', open);
    launcherButton?.classList.toggle('active', open);
    launcher?.setAttribute('aria-hidden', String(!open));
  };

  launcherButton?.addEventListener('click', (event) => {
    event.stopPropagation();
    setLauncher(!launcher.classList.contains('open'));
  });

  desktopStage?.addEventListener('click', (event) => {
    if (!launcher?.contains(event.target) && !launcherButton?.contains(event.target)) setLauncher(false);
  });

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      setLauncher(false);
      mobileNav?.classList.remove('open');
      document.body.classList.remove('menu-open');
      menuButton?.setAttribute('aria-expanded', 'false');
    }
  });

  const updateClock = () => {
    if (!clock) return;
    const now = new Date();
    clock.textContent = new Intl.DateTimeFormat(undefined, {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false
    }).format(now);
  };
  updateClock();
  setInterval(updateClock, 30000);

  document.querySelectorAll('[data-year]').forEach((node) => {
    node.textContent = String(new Date().getFullYear());
  });

  // Reveal animations on scroll
  const revealNodes = document.querySelectorAll('.reveal');
  if ('IntersectionObserver' in window && !window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add('visible');
          observer.unobserve(entry.target);
        }
      });
    }, { threshold: 0.10, rootMargin: '0px 0px -20px' });
    revealNodes.forEach((node) => observer.observe(node));
  } else {
    revealNodes.forEach((node) => node.classList.add('visible'));
  }

  // Also make sure above-the-fold nodes are visible immediately
  setTimeout(() => {
    document.querySelectorAll('.hero .reveal, #main > section:first-of-type .reveal').forEach((node) => {
      node.classList.add('visible');
    });
  }, 50);

  if (desktopStage && window.matchMedia('(pointer:fine)').matches) {
    desktopStage.addEventListener('pointermove', (event) => {
      const box = desktopStage.getBoundingClientRect();
      const x = (event.clientX - box.left) / box.width - 0.5;
      const y = (event.clientY - box.top) / box.height - 0.5;
      desktopStage.style.setProperty('--pointer-x', x.toFixed(3));
      desktopStage.style.setProperty('--pointer-y', y.toFixed(3));
    });
  }

  // Interactive Desktop Mock-up Handlers
  const launcherInput = document.querySelector('[data-launcher-input]');
  const launcherGrid = document.querySelector('[data-launcher-grid]');

  launcherInput?.addEventListener('input', (e) => {
    const term = e.target.value.toLowerCase().trim();
    if (!launcherGrid) return;
    launcherGrid.querySelectorAll('button').forEach((btn) => {
      const name = (btn.getAttribute('data-app-name') || btn.textContent).toLowerCase();
      btn.style.display = name.includes(term) ? '' : 'none';
    });
  });

  const workspaceDots = document.querySelectorAll('[data-workspace-dots] .ws-dot');
  const desktopStatusText = document.querySelector('[data-desktop-status-text]');

  workspaceDots.forEach((dot) => {
    dot.addEventListener('click', (e) => {
      e.stopPropagation();
      workspaceDots.forEach((d) => d.classList.remove('active'));
      dot.classList.add('active');
      const wsNum = dot.getAttribute('data-ws');
      if (desktopStatusText) {
        desktopStatusText.textContent = t('ws_active', { n: wsNum });
      }
    });
  });

  const appDockBtns = document.querySelectorAll('[data-app-dock] button');
  appDockBtns.forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const appName = btn.getAttribute('data-app');
      if (desktopStatusText) {
        desktopStatusText.textContent = t('app_ready', { app: appName });
      }
    });
  });
})();
