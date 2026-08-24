// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
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
      nav_experience: 'Interface',
      nav_foundation: 'Foundation',
      nav_progress: 'Progress',
      nav_design: 'Design',
      nav_roadmap: 'Roadmap',
      nav_blueprint: 'Blueprint',
      nav_faq: 'FAQ',
      nav_partners: 'Partners',
      nav_github: 'GitHub',
      btn_view_blueprint: 'Blueprint',
      btn_back_home: '← Back to home',
      part_badge: 'Partnership & Support Hub',
      part_title: 'Partners, Contact & Donations',
      part_lead: 'Get in touch with creator Stanislav Saveliev, explore hardware & software partnerships, or support Cybou’s independent open-source development.',
      part_contact_head: 'Official Contact',
      part_contact_text: 'For general inquiries, press, security disclosures, distributions collaboration, or hardware support:',
      part_donate_head: 'Support & Donations',
      part_donate_text: 'Cybou is an independent open-source project. Your support funds reproducible builds, the Rust/WebAssembly interface, Mind runtime engineering, and fault-recovery testing.',
      part_crypto_head: 'Crypto Donations',
      part_crypto_text: 'Support Cybou directly via cryptocurrency:',
      faq_h2_1: 'Frequently asked',
      faq_h2_2: 'questions.',
      faq_q1: 'What is Cybou?',
      faq_a1: 'Cybou is an experimental agent-native operating environment. A persistent local runtime called Mind owns durable memory, identity, commitments, lifecycle, health, and evidence, while models, agents, tools, and user interfaces stay replaceable around it.',
      faq_q2: 'How is that different from adding a chatbot to Linux?',
      faq_a2: 'A model is not Mind. An agent is not Mind. A tool protocol is not an authorization boundary. Cybou makes memory, identity, health, lifecycle, and recovery explicit system services with named owners, so no model can become the owner of continuity or authority.',
      faq_q3: 'Does Cybou require cloud services or send telemetry?',
      faq_a3: 'No. The current runtime and interface need no cloud account, API key, or hosted AI service, and Cybou implements no telemetry. Remote models may become available later, but only behind explicit context, sensitivity, egress, and cost policy.',
      faq_q4: 'What is the interface built with?',
      faq_a4: 'One Rust/WebAssembly frontend called Living Canvas, served over a Rust gateway that exposes typed session, snapshot, and resumable event routes and no mutation route. The interface is a projection of Mind and never a second owner of state, which is what makes it replaceable.',
      faq_q5: 'What is not implemented yet?',
      faq_a5: 'Cybou ships no language model, no agent or worker runtime, no tool broker, no privileged action executor, and no autonomous security control plane. The model broker exists as a faculty with no worker behind it. These are planned boundaries and are never described as done.',
      faq_q6: 'What open-source licenses apply?',
      faq_a6: 'Cybou code and most documentation are licensed under MIT. Original visual assets are licensed under CC BY-SA 4.0. The repository follows the REUSE specification. Copyright (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Pre-release · Server-first · No model shipped',
      hero_h1_1: 'Linux that understands',
      hero_h1_2: 'and operates itself.',
      hero_lead: 'Cybou is an experimental agent-native operating environment for a server, VPS or container — a machine that runs unattended and is expected to look after itself. It watches its own state, diagnoses what is wrong, and shows you the readings it reasoned from. With the network down and no model loaded.',
      hero_btn_explore: 'Build Cybou',
      hero_btn_blueprint: 'Read Technical Blueprint',
      tag_unplugged: 'Works unplugged',
      tag_built_with: 'Rust · WebAssembly',
      tag_ai: 'Evidence, not assertions',
      tag_telemetry: 'Reports nothing to us',
      desktop_welcome: 'Welcome to Cybou',
      desktop_sub: 'A quiet system, ready when you are.',
      launcher_search: 'Search applications',
      app_files: 'Files',
      app_web: 'Web',
      app_code: 'Code',
      app_settings: 'Settings',
      gen_status: 'System is healthy',
      visual_caption: 'Interactive interface concept — click the Cybou mark',
      concept_badge: 'Concept preview',
      exp_h2_1: 'It watches the machine.',
      exp_h2_2: 'Then it explains itself.',
      exp_lead: 'Load, memory and I/O pressure, filesystem and inode usage, open descriptors, failed units — plus the certificates, services and backups you declare. What it concludes from them is a hypothesis carrying the readings behind it, never a fact. Ask why it thinks so and it shows you the numbers, not a sentence a model composed.',
      feat1_title: 'Living Canvas',
      feat1_desc: 'One Rust/WebAssembly frontend, compiled from a single source, delivered as a content-hashed artifact from the same origin as its own API.',
      feat2_title: 'A read-only boundary',
      feat2_desc: 'The gateway binds to loopback, answers typed session, snapshot, and event requests under a bounded budget, and exposes no mutation route at all.',
      feat3_title: 'Designed as a system',
      feat3_desc: 'Color, geometry, motion, login, and wallpaper share one coherent visual grammar in both dark and light.',
      found_h2_1: 'It can propose.',
      found_h2_2: 'It cannot act.',
      found_lead: 'Cybou forms typed remediation proposals, criticises each one against the finding it claims to relieve, and puts it to a standing policy that grants nothing until you configure it. Nothing in this build can carry a privileged operation out. The boundary was written before the executor on purpose — written the other way round, an executor arrives with the decision to act already inside it.',
      p1_head: 'Proposed',
      p1_text: 'A typed operation from a closed set, never shell text.',
      p2_head: 'Criticised',
      p2_text: 'Checked against the finding it claims to relieve.',
      p3_head: 'Authorized',
      p3_text: 'Refused until a policy you set permits it.',
      gen_console_title: 'System states',
      gen147_desc: 'Current · Rust interface foundation',
      gen_active: 'Active',
      gen146_desc: 'Security update · verified',
      gen_yesterday: 'Yesterday',
      gen145_desc: 'Before graphics configuration',
      gen_aug2: 'Aug 2',
      prog_h2_1: 'Measured execution.',
      prog_h2_2: 'Verified milestones.',
      prog_lead: 'The substrate carries grounded perception, epistemic projection, crash-safe erasure, governed context delivery, structured meaning, bounded Body telemetry, and a remediation boundary with no executor behind it. Each layer had to be verifiable before the next was allowed to depend on it.',
      metric_gate_a: 'Isolated Mind services with explicit typed ownership over D-Bus',
      metric_tasks: 'One gate script: format, lint, native and browser tests, document and layering validators, live-bus integration',
      metric_contrast: 'One locked Cargo workspace with unsafe code forbidden across it',
      status_development: 'Implemented runtime',
      status_runtime: 'Verified baseline',
      status_verified: 'Migration in progress',
      evidence_state: 'Current State',
      evidence_state_desc: 'Implemented behavior and limitations',
      evidence_tests: 'Testing evidence',
      evidence_tests_desc: 'Process, reboot, fault and KVM gates',
      evidence_source: 'Source repository',
      evidence_source_desc: 'Inspect the code and development history',
      design_sub: 'A visual system shaped by depth, negative space and a single line of light.',
      mark_desc: 'An open arc around a focused center: a system with room to evolve.',
      type_head: 'Calm, precise, human.',
      type_desc: 'System sans-serif typography keeps the site independent from remote services.',
      princ_h2_1: 'Replaceable where it can be.',
      princ_h2_2: 'Owned where it must be.',
      pr1_head: 'Quiet by default',
      pr1_desc: 'No animated mascot, no glowing assistant, no feed competing for attention.',
      pr2_head: 'Surfaces are replaceable',
      pr2_desc: 'Interfaces, models and tools can be swapped without touching memory, identity or policy.',
      pr3_head: 'Private by construction',
      pr3_desc: 'No telemetry, no account requirement and no runtime asset downloads.',
      pr4_head: 'Built in stages',
      pr4_desc: 'Each layer must be verifiable before the next one is allowed to depend on it.',
      road_h2_1: 'Build the body.',
      road_h2_2: 'Then grow the mind.',
      rm1_title: 'Mind substrate',
      rm2_title: 'Web-first Rust surface',
      rm3_title: 'Body observation and diagnosis',
      rm4_title: 'Governed action and its outcome',
      rm1_sub: 'Durable history · Identity · Erasure · Lifecycle',
      rm2_sub: 'Protocol crates · Gateway · Living Canvas',
      rm3_sub: 'Telemetry · Findings · Projections · Meaning',
      rm4_sub: 'Executor · Independent re-observation · Agents',
      rm_complete: 'Complete',
      rm_inprogress: 'In progress',
      bp_banner_kicker: 'The project starts here',
      bp_banner_h2_1: 'A real operating system,',
      bp_banner_h2_2: 'built one trusted layer at a time.',
      bp_banner_desc: 'The blueprint defines the Mind runtime and its typed ownership, the Rust/WebAssembly interface path, the observation and diagnosis layer, the action boundary, the verified implementation gates, and the cognitive roadmap.',
      bp_banner_btn: 'Read Technical Whitepaper',
      btn_top: 'Back to top',
      footer_sub: 'Agent-native environment · Typed local Mind · Rust and WebAssembly interface.',
      bp_badge: 'Technical Blueprint · Current Architecture & Roadmap',
      bp_title: 'Cybou: A Governed Cognitive Runtime with a Replaceable Interface',
      bp_lead: 'The current whitepaper for Cybou’s Mind runtime, its ownership and failure model, the observation and diagnosis layer, the action boundary with no executor behind it, the verified implementation gates, and the cognitive roadmap.',
      btn_print_pdf: 'Export to PDF / Print',
      bp_sec1_title: '1. Executive Summary & Core Vision',
      toc_vision: 'Vision',
      toc_architecture: 'Architecture',
      toc_safety: 'Safety boundaries',
      toc_implementation: 'Implementation',
      toc_security: 'Security & licensing',
      toc_faq: 'FAQ',
      bp_sec1_p1: 'Cybou is an experimental agent-native operating environment with independently testable layers: a reproducible system foundation, Mind — a local typed runtime for durable biography, identity, commitments, prediction, bounded attention, health and recovery — and a replaceable Rust/WebAssembly interface. It is not a chatbot and makes no claim of consciousness.',
      bp_layer1_title: 'Layer 1: Reproducible Body',
      bp_layer1_desc: 'Debian 13 packages and systemd user units, deployed to a server over SSH, with explicit build and recovery gates. Debian is the build, verification and deployment environment, not a host that stands in for another one.',
      bp_layer2_title: 'Layer 2: Typed Mind Runtime',
      bp_layer2_desc: 'Isolated systemd user services communicate through typed D-Bus contracts. Event1 is the canonical Journal boundary; Presence is a projection, not a second owner. Language models and privileged execution remain absent.',
      bp_layer3_title: 'Layer 3: Replaceable Interface',
      bp_layer3_desc: 'A Rust workspace carries the protocol, fabric codec, runtime and storage paths, and web contracts. A read-only gateway on loopback projects Presence into typed JSON and a bounded event stream, and Living Canvas renders it as WebAssembly in a browser or desktop shell.',
      bp_sec2_title: '2. Architecture & Technical Stack',
      bp_sec2_p1: 'Cybou separates Body, Mind, and Presence. Durable state belongs to explicit owners; every surface is a cache and presentation boundary. Cross-owner commands are bounded and fail closed.',
      bp_stack_title: 'Core System Stack',
      bp_sec3_title: '3. Cognitive Contracts and Safety Boundaries',
      bp_sec3_p1: 'The implemented substrate keeps cognition inspectable without turning a model, UI, or coordinator into an unbounded owner. Future capabilities must preserve these contracts:',
      bp_ai_point1: '<strong>Ownership before intelligence:</strong> model ≠ identity, UI ≠ Mind, attention ≠ biography, and proposal ≠ authorization.',
      bp_ai_point2: '<strong>Durability before visibility:</strong> state is projected only after its owner commits it; consolidation adds evidence-linked outcomes and never rewrites history.',
      bp_ai_point3: '<strong>Bounded degradation:</strong> Health1 publishes typed deficits and recovery progress; compound reads and mutations share one monotonic deadline.',
      bp_ai_point4: '<strong>No hidden agency:</strong> the interface may only read; M8 language is optional and replaceable, and M9 learning, M10 action, and M11 agents remain separate governed boundaries.',
      bp_sec4_title: '4. Implementation Status & Progress Matrix',
      bp_sec4_p1: 'Development progress is tracked rigorously across milestone phases and verified against automated acceptance gates.',
      bp_sec5_title: '5. Security, Privacy & Licensing',
      bp_sec5_p1: 'Cybou is free software built for sovereignty and privacy:',
      nav_partners_footer: 'Partners & Donations',
      kicker_interface: '01 · Interface',
      kicker_foundation: '02 · Foundation',
      kicker_progress: '03 · Live implementation progress',
      kicker_design: '04 · Design language',
      kicker_principles: '05 · Principles',
      kicker_roadmap: '06 · Roadmap',
      kicker_faq: '07 · Questions & architecture',
      label_symbol: 'Symbol',
      label_palette: 'Palette',
      label_typography: 'Typography',
      palette_name: 'Mineral dark · Aurora mint',
      desktop_stable: 'Stable',
      ws_active: 'Workspace {n} — active',
      app_ready: '{app} ready',
      bp_meta_foundation: 'Foundation:',
      bp_meta_foundation_v: 'Locked dependencies, atomic generations, rollback',
      bp_meta_interface: 'Interface:',
      bp_meta_interface_v: 'Rust / WebAssembly (Living Canvas)',
      bp_meta_status: 'Status:',
      bp_meta_status_v: 'Substrate, observation and diagnosis verified · action boundary built, executor absent',
      bp_meta_updated: 'Updated:',
      bp_meta_updated_v: 'August 2026',
      btn_view_state: 'View Current State',
      bp_stack1: '<strong>Base System:</strong> Debian 13 is the build, verification and deployment target. Cybou is developed server-first: a VPS, VM or container is the primary environment, not a workstation.',
      bp_stack2: '<strong>Rust Workspace:</strong> One locked Cargo workspace holding the protocol and fabric codec, runtime and storage paths, replacement organ slices, web contracts, the read-only gateway, and the Living Canvas WebAssembly frontend.',
      bp_stack3: '<strong>Delivery:</strong> The gateway binds to loopback, serves the content-hashed frontend from its own origin, and exposes typed session, snapshot, and bounded event-stream routes with no mutation route.',
      bp_stack4: '<strong>Session:</strong> An opt-in Wayland session gives the frontend a single surface through a minimal compositor and an ephemeral browser runtime profile. It is installed disabled and is unproven on hardware with a seat; the browser is the supported surface.',
      bp_stack5: '<strong>Theme & Aesthetics:</strong> Cybou Horizon global theme, dark/light color tokens, and one coherent visual grammar across login, windows, and wallpaper.',
      bp_stack6: '<strong>Build Output:</strong> Debian packages and systemd user units, deployed to a server over SSH and reached through a browser.',
      bp_stack7: '<strong>Mind Runtime:</strong> Rust user services activated by systemd and addressed through typed D-Bus interfaces, in one workspace where unsafe code is forbidden.',
      bp_stack8: '<strong>Canonical Memory:</strong> One event daemon is the only Journal writer; Journal v3 preserves causal metadata verification while supporting crash-safe payload erasure.',
      bp_stack9: '<strong>Continuity:</strong> Identity, intentions, lifecycle runs, health snapshots, and event consumer progress persist under versioned schemas and atomic writes.',
      bp_stack10: '<strong>Failure Model:</strong> Optional-owner loss degrades only dependent capabilities; required-owner loss fails mutations closed without inventing state.',
      bp_topology_title: 'Process and ownership topology',
      th_milestone: 'Milestone',
      th_status: 'Status',
      th_capability: 'Capability',
      th_boundary: 'Implemented boundary',
      th_evidence: 'Primary evidence',
      tag_done: 'DONE',
      tag_inprogress: 'IN PROGRESS',
      tag_planned: 'PLANNED',
      row_m0_cap: 'Green reproducible baseline',
      row_m0_bound: 'Locked Cargo workspace, formatting, licensing, metadata, documentation and interface gates',
      row_m0_ev: 'One gate script over every check, reporting which step failed',
      row_m14_cap: 'Accepted memory and isolated organs',
      row_m14_bound: 'Single Event1 writer, isolated services, remote Presence proxy',
      row_m14_ev: 'Protocol, process, event, UI API and VM tests',
      row_m5_cap: 'Continuity and consolidation lifecycle',
      row_m5_bound: 'Persistent run state, Lifecycle1, deterministic owner effects, restart and reboot recovery',
      row_m5_ev: 'Lifecycle continuity and split-commit fault gates',
      row_m6_cap: 'Health, scheduling and recovery',
      row_m6_bound: 'Capability graph, Health1, homeostasis v2, evidence-bound scheduling, degraded UI contract',
      row_m6_ev: 'Recovery boundary and process fault matrix',
      row_p67_cap: 'Bounded Presence orchestration',
      row_p67_bound: 'One monotonic budget for every compound Presence read and mutation',
      row_p67_ev: 'Bounded RPC, suspended-owner process tests and KVM continuity',
      row_m7_cap: 'Grounded cognition and governed context',
      row_m7_bound: 'Perception, epistemics, Journal v3 erasure, sensitivity, associative projection, and governed delivery',
      row_m7_ev: 'Focused unit, process, scale, retention and disclosure gates',
      row_body_cap: 'Body observation, diagnosis and projection',
      row_body_bound: 'Bounded transient telemetry that never enters the biography, findings that carry the readings behind them, projections from robust statistics, and named resources an operator declares',
      row_body_ev: 'Detector, projection, watchlist and end-to-end walkthrough gates',
      row_w01_cap: 'Rust foundation and read-only web boundary',
      row_w01_bound: 'Locked Cargo workspace, protocol, fabric, runtime and web-contract crates, loopback gateway with typed session, snapshot, and resumable event stream',
      row_w01_ev: 'Native tests, strict lints, WebAssembly build and release frontend gates',
      row_w2_cap: 'One interface, served to a browser',
      row_w2_bound: 'Shared WebAssembly frontend behind a loopback gateway, reached over a reverse proxy; the opt-in single-surface desktop session installs disabled',
      row_w2_ev: 'Browser tests and gateway gates; the desktop session is unproven on hardware with a seat',
      row_m8_cap: 'Optional language faculty',
      row_m8_bound: 'Replaceable model behind typed context and proposal contracts',
      row_m8_ev: 'No model is shipped today',
      row_m913_cap: 'Learning, governed action, agents and security',
      row_m913_bound: 'The action boundary is built — typed proposals, criticism, standing policy, authorization — and nothing behind it can execute. Learned artifacts, agents, tools and security remain separate governed boundaries.',
      row_m913_ev: 'Proposal and authorization gates pass; no executor exists, deliberately',
      bp_lic1: '<strong>Code License:</strong> MIT License for the Rust workspace, build and deployment scripts, web assets, and core logic.',
      bp_lic2: '<strong>Visual Assets:</strong> Creative Commons Attribution-ShareAlike 4.0 for logos, wallpapers, and desktop themes.',
      bp_lic3: '<strong>Trust Boundaries:</strong> Same-user local IPC is not treated as a complete capability-security boundary; future privileged action requires a separate authorization layer.',
      bp_lic4: '<strong>Interface Boundary:</strong> The gateway refuses to bind anything but loopback, is read-only by construction, and applies no-store and browser-security headers. A deployment reached from outside the machine puts a reverse proxy in front of it and requires sign-in; serving strangers is a mode the unit sets, never the default.',
      bp_lic5: '<strong>Privacy:</strong> Event envelopes carry explicit privacy and sensitivity axes. Sensitive payloads are sealed with per-contribution keys, and crash-safe transitive erasure destroys them. A backup that also captured the key store is outside that guarantee, which is tested rather than assumed. Automatic expiry and replication remain open.',
      bp_lic6: '<strong>Current Cloud Boundary:</strong> The runtime and interface require no hosted AI service or API key, and no language model is shipped today.',
      bp_lic7: '<strong>Security Documents:</strong> The repository maintains explicit threat and privacy models under version control.',
      bp_sources_title: 'Canonical sources and claim authority',
      bp_sources_lead: 'This web whitepaper is a readable summary. Exact implementation claims are governed by the repository:',
      bp_src1: '— authoritative implemented boundary.',
      bp_src2: '— topology, ownership, and long-term model.',
      bp_src3: '— process, reboot, fault, and KVM evidence.',
      bp_src4: '— detailed implementation record and current work.',
      bp_sec6_title: '6. Frequently Asked Questions',
      skip_link: 'Skip to content',
      metric_v_services: 'Typed Mind services',
      metric_v_tests: 'One gate, no partial pass',
      metric_v_crates: 'Rust workspace'
    },
    fr: {
      nav_experience: 'Interface',
      nav_foundation: 'Fondation',
      nav_progress: 'Progression',
      nav_design: 'Design',
      nav_roadmap: 'Feuille de route',
      nav_blueprint: 'Blueprint',
      nav_faq: 'FAQ',
      nav_partners: 'Partenaires',
      nav_github: 'GitHub',
      btn_view_blueprint: 'Blueprint',
      btn_back_home: '← Retour à l’accueil',
      part_badge: 'Partenariats & soutien',
      part_title: 'Partenaires, contact et dons',
      part_lead: 'Contactez le créateur Stanislav Saveliev, explorez des partenariats matériels et logiciels, ou soutenez le développement open source indépendant de Cybou.',
      part_contact_head: 'Contact officiel',
      part_contact_text: 'Pour toute demande générale, presse, divulgation de sécurité, collaboration entre distributions ou support matériel :',
      part_donate_head: 'Soutien et dons',
      part_donate_text: 'Cybou est un projet open source indépendant. Votre soutien finance les builds reproductibles, l’interface Rust/WebAssembly, l’ingénierie du runtime Mind et les tests de reprise après panne.',
      part_crypto_head: 'Dons en crypto',
      part_crypto_text: 'Soutenez directement Cybou en cryptomonnaie :',
      faq_h2_1: 'Questions',
      faq_h2_2: 'fréquentes.',
      faq_q1: 'Qu’est-ce que Cybou ?',
      faq_a1: 'Cybou est un environnement d’exploitation expérimental conçu pour les agents. Un runtime local persistant nommé Mind détient la mémoire durable, l’identité, les engagements, le cycle de vie, la santé et les preuves, tandis que les modèles, agents, outils et interfaces restent remplaçables autour de lui.',
      faq_q2: 'En quoi est-ce différent d’un chatbot ajouté à Linux ?',
      faq_a2: 'Un modèle n’est pas Mind. Un agent n’est pas Mind. Un protocole d’outils n’est pas une frontière d’autorisation. Cybou fait de la mémoire, de l’identité, de la santé, du cycle de vie et de la reprise des services système explicites avec des propriétaires nommés : aucun modèle ne peut devenir propriétaire de la continuité ni de l’autorité.',
      faq_q3: 'Cybou exige-t-il le cloud ou envoie-t-il de la télémétrie ?',
      faq_a3: 'Non. Le runtime et l’interface actuels ne demandent aucun compte cloud, clé d’API ni service d’IA hébergé, et Cybou n’implémente aucune télémétrie. Des modèles distants pourront exister plus tard, mais uniquement derrière une politique explicite de contexte, de sensibilité, de sortie réseau et de coût.',
      faq_q4: 'Avec quoi l’interface est-elle construite ?',
      faq_a4: 'Un unique frontend Rust/WebAssembly nommé Living Canvas, servi par une passerelle Rust qui expose des routes typées de session, d’instantané et d’événements reprenables, et aucune route de mutation. L’interface est une projection de Mind et jamais un second propriétaire de l’état, ce qui la rend remplaçable.',
      faq_q5: 'Qu’est-ce qui n’est pas encore implémenté ?',
      faq_a5: 'Cybou ne livre aucun modèle de langage, aucun runtime d’agents ou de workers, aucun courtier de modèles ou d’outils, aucun exécuteur d’actions privilégiées et aucun plan de contrôle de sécurité autonome. L’authentification au démarrage du bureau reste également ouverte. Ce sont des frontières planifiées, jamais décrites comme achevées.',
      faq_q6: 'Quelles licences open source s’appliquent ?',
      faq_a6: 'Le code et la majeure partie de la documentation de Cybou sont sous licence MIT. Les ressources visuelles originales sont sous CC BY-SA 4.0. Le dépôt suit la spécification REUSE. Copyright (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Pré-version · pensé pour le serveur · aucun modèle livré',
      hero_h1_1: 'Un Linux qui se comprend',
      hero_h1_2: 'et qui s’administre.',
      hero_lead: 'Cybou est un environnement d’exploitation expérimental conçu pour les agents, destiné à un serveur, un VPS ou un conteneur — une machine qui tourne sans surveillance et dont on attend qu’elle veille sur elle-même. Elle observe son propre état, diagnostique ce qui ne va pas et montre les mesures dont elle est partie. Réseau coupé, aucun modèle chargé.',
      hero_btn_explore: 'Compiler Cybou',
      hero_btn_blueprint: 'Lire le blueprint technique',
      tag_unplugged: 'Fonctionne débranché',
      tag_built_with: 'Rust · WebAssembly',
      tag_ai: 'Des mesures, pas des affirmations',
      tag_telemetry: 'Ne nous rapporte rien',
      desktop_welcome: 'Bienvenue sur Cybou',
      desktop_sub: 'Un système calme, prêt quand vous l’êtes.',
      launcher_search: 'Rechercher des applications',
      app_files: 'Fichiers',
      app_web: 'Web',
      app_code: 'Code',
      app_settings: 'Réglages',
      gen_status: 'Le système est sain',
      visual_caption: 'Concept d’interface interactif — cliquez sur la marque Cybou',
      concept_badge: 'Aperçu conceptuel',
      exp_h2_1: 'Elle observe la machine.',
      exp_h2_2: 'Puis elle s’explique.',
      exp_lead: 'Charge, pression mémoire et E/S, occupation du système de fichiers et des inodes, descripteurs ouverts, unités en échec — et les certificats, services et sauvegardes que vous déclarez. Ce qu’elle en conclut est une hypothèse qui porte les mesures dont elle vient, jamais un fait. Demandez-lui pourquoi : elle montre les chiffres, pas une phrase composée par un modèle.',
      feat1_title: 'Living Canvas',
      feat1_desc: 'Un frontend Rust/WebAssembly unique, compilé depuis une seule source et livré comme artefact au hachage de contenu, depuis la même origine que son API.',
      feat2_title: 'Une frontière en lecture seule',
      feat2_desc: 'La passerelle écoute uniquement en local, répond aux demandes typées de session, d’instantané et d’événements sous un budget borné, et n’expose aucune route de mutation.',
      feat3_title: 'Conçu comme un système',
      feat3_desc: 'Couleur, géométrie, mouvement, connexion et fond d’écran partagent une même grammaire visuelle, en clair comme en sombre.',
      found_h2_1: 'Elle peut proposer.',
      found_h2_2: 'Elle ne peut pas agir.',
      found_lead: 'Cybou formule des propositions de remédiation typées, critique chacune face au constat qu’elle prétend soulager, et la soumet à une politique permanente qui n’accorde rien tant que vous ne l’avez pas configurée. Rien dans cette version ne peut exécuter une opération privilégiée. La frontière a été écrite avant l’exécuteur, délibérément : dans l’autre sens, un exécuteur arrive avec la décision d’agir déjà à l’intérieur.',
      p1_head: 'Proposé',
      p1_text: 'Une opération typée issue d’un ensemble fermé, jamais du texte shell.',
      p2_head: 'Critiqué',
      p2_text: 'Confronté au constat qu’il prétend soulager.',
      p3_head: 'Autorisé',
      p3_text: 'Refusé tant qu’une politique que vous avez définie ne l’autorise pas.',
      gen_console_title: 'États du système',
      gen147_desc: 'Actuelle · fondation de l’interface Rust',
      gen_active: 'Active',
      gen146_desc: 'Mise à jour de sécurité · vérifiée',
      gen_yesterday: 'Hier',
      gen145_desc: 'Avant la configuration graphique',
      gen_aug2: '2 août',
      prog_h2_1: 'Exécution mesurée.',
      prog_h2_2: 'Jalons vérifiés.',
      prog_lead: 'Le substrat porte la perception ancrée, la projection épistémique, l’effacement sûr en cas de panne, la livraison gouvernée du contexte, le sens structuré, la télémétrie bornée du Corps, et une frontière de remédiation sans exécuteur derrière elle. Chaque couche devait être vérifiable avant que la suivante n’ait le droit d’en dépendre.',
      metric_gate_a: 'Des services Mind isolés, avec une propriété typée explicite, via D-Bus',
      metric_tasks: 'Un seul script de contrôle : format, lint, tests natifs et navigateur, validateurs de documents et de couches, intégration sur bus vivant',
      metric_contrast: 'Un espace de travail Cargo verrouillé, où le code unsafe est interdit partout',
      status_development: 'Runtime implémenté',
      status_runtime: 'Base vérifiée',
      status_verified: 'Migration en cours',
      evidence_state: 'État actuel',
      evidence_state_desc: 'Comportement implémenté et limites',
      evidence_tests: 'Preuves de test',
      evidence_tests_desc: 'Processus, redémarrage, pannes et KVM',
      evidence_source: 'Dépôt source',
      evidence_source_desc: 'Inspecter le code et l’historique',
      design_sub: 'Un système visuel façonné par la profondeur, l’espace négatif et une seule ligne de lumière.',
      mark_desc: 'Un arc ouvert autour d’un centre net : un système qui garde de la place pour évoluer.',
      type_head: 'Calme, précis, humain.',
      type_desc: 'La typographie sans-serif système garde le site indépendant de tout service distant.',
      princ_h2_1: 'Remplaçable là où c’est possible.',
      princ_h2_2: 'Détenu là où c’est nécessaire.',
      pr1_head: 'Silencieux par défaut',
      pr1_desc: 'Pas de mascotte animée, pas d’assistant lumineux, pas de flux qui réclame l’attention.',
      pr2_head: 'Des surfaces remplaçables',
      pr2_desc: 'Interfaces, modèles et outils peuvent être échangés sans toucher à la mémoire, à l’identité ni à la politique.',
      pr3_head: 'Privé par construction',
      pr3_desc: 'Aucune télémétrie, aucun compte requis, aucun téléchargement d’actifs à l’exécution.',
      pr4_head: 'Construit par étapes',
      pr4_desc: 'Chaque couche doit être vérifiable avant que la suivante ait le droit d’en dépendre.',
      road_h2_1: 'Construire le corps.',
      road_h2_2: 'Puis faire grandir l’esprit.',
      rm1_title: 'Socle Mind',
      rm2_title: 'Surface Rust orientée web',
      rm3_title: 'Observation et diagnostic du Corps',
      rm4_title: 'Action gouvernée et son résultat',
      rm1_sub: 'Historique durable · Identité · Effacement · Cycle de vie',
      rm2_sub: 'Crates de protocole · Passerelle · Living Canvas',
      rm3_sub: 'Télémétrie · Constats · Projections · Sens',
      rm4_sub: 'Exécuteur · Réobservation indépendante · Agents',
      rm_complete: 'Terminé',
      rm_inprogress: 'En cours',
      bp_banner_kicker: 'Le projet commence ici',
      bp_banner_h2_1: 'Un vrai système d’exploitation,',
      bp_banner_h2_2: 'construit une couche de confiance à la fois.',
      bp_banner_desc: 'Le blueprint définit le runtime Mind et sa propriété typée, le chemin d’interface Rust/WebAssembly, la couche d’observation et de diagnostic, la frontière d’action, les portes d’implémentation vérifiées et la feuille de route cognitive.',
      bp_banner_btn: 'Lire le livre blanc technique',
      btn_top: 'Retour en haut',
      footer_sub: 'Environnement pour agents · Mind local typé · interface Rust et WebAssembly.',
      bp_badge: 'Blueprint technique · architecture et feuille de route',
      bp_title: 'Cybou : un runtime cognitif gouverné avec une interface remplaçable',
      bp_lead: 'Le livre blanc actuel du runtime Mind, de son modèle de propriété et de panne, de la couche d’observation et de diagnostic, de la frontière d’action sans exécuteur derrière elle, des portes d’implémentation vérifiées et de la feuille de route cognitive.',
      btn_print_pdf: 'Exporter en PDF / Imprimer',
      bp_sec1_title: '1. Résumé et vision',
      toc_vision: 'Vision',
      toc_architecture: 'Architecture',
      toc_safety: 'Frontières de sûreté',
      toc_implementation: 'Implémentation',
      toc_security: 'Sécurité et licences',
      toc_faq: 'FAQ',
      bp_sec1_p1: 'Cybou est un environnement d’exploitation expérimental pour agents, composé de couches testables indépendamment : une fondation système reproductible, Mind — un runtime local typé pour la biographie durable, l’identité, les engagements, la prédiction, l’attention bornée, la santé et la reprise — et une interface Rust/WebAssembly remplaçable. Ce n’est pas un chatbot et il ne prétend à aucune conscience.',
      bp_layer1_title: 'Couche 1 : corps reproductible',
      bp_layer1_desc: 'Paquets Debian 13 et unités utilisateur systemd, déployés sur un serveur par SSH, avec des portes explicites de build et de reprise. Debian est l’environnement de build, de vérification et de déploiement, et non un hôte qui en remplace un autre.',
      bp_layer2_title: 'Couche 2 : runtime Mind typé',
      bp_layer2_desc: 'Des services utilisateur systemd isolés communiquent par contrats D-Bus typés. Event1 est la frontière canonique du Journal ; Presence est une projection, pas un second propriétaire. Aucun modèle de langage ni exécution privilégiée.',
      bp_layer3_title: 'Couche 3 : interface remplaçable',
      bp_layer3_desc: 'Un espace de travail Rust porte le protocole, le codec fabric, les chemins d’exécution et de stockage, et les contrats web. Une passerelle en lecture seule sur la boucle locale projette Presence en JSON typé et en flux d’événements borné, et Living Canvas l’affiche en WebAssembly dans un navigateur ou un shell de bureau.',
      bp_sec2_title: '2. Architecture et pile technique',
      bp_sec2_p1: 'Cybou sépare le Corps, Mind et Presence. L’état durable appartient à des propriétaires explicites ; toute surface n’est qu’un cache et une frontière de présentation. Les commandes inter-propriétaires sont bornées et échouent fermées.',
      bp_stack_title: 'Pile système principale',
      bp_sec3_title: '3. Contrats cognitifs et frontières de sûreté',
      bp_sec3_p1: 'Le substrat implémenté garde la cognition inspectable sans transformer un modèle, une UI ou un coordinateur en propriétaire illimité. Les capacités futures doivent préserver ces contrats :',
      bp_ai_point1: '<strong>La propriété avant l’intelligence :</strong> modèle ≠ identité, UI ≠ Mind, attention ≠ biographie, proposition ≠ autorisation.',
      bp_ai_point2: '<strong>La durabilité avant la visibilité :</strong> l’état n’est projeté qu’après validation par son propriétaire ; la consolidation ajoute des résultats liés aux preuves et ne réécrit jamais l’histoire.',
      bp_ai_point3: '<strong>Dégradation bornée :</strong> Health1 publie des déficits typés et la progression de reprise ; les lectures et mutations composées partagent une seule échéance monotone.',
      bp_ai_point4: '<strong>Aucune agentivité cachée :</strong> l’interface ne peut que lire ; le langage M8 est optionnel et remplaçable, et l’apprentissage M9, l’action M10 et les agents M11 restent des frontières gouvernées distinctes.',
      bp_sec4_title: '4. État d’implémentation et matrice de progression',
      bp_sec4_p1: 'La progression est suivie rigoureusement par jalons et vérifiée par des portes d’acceptation automatisées.',
      bp_sec5_title: '5. Sécurité, vie privée et licences',
      bp_sec5_p1: 'Cybou est un logiciel libre conçu pour la souveraineté et la vie privée :',
      nav_partners_footer: 'Partenaires & dons',
      kicker_interface: '01 · Interface',
      kicker_foundation: '02 · Fondation',
      kicker_progress: '03 · Progression réelle',
      kicker_design: '04 · Langage visuel',
      kicker_principles: '05 · Principes',
      kicker_roadmap: '06 · Feuille de route',
      kicker_faq: '07 · Questions & architecture',
      label_symbol: 'Symbole',
      label_palette: 'Palette',
      label_typography: 'Typographie',
      palette_name: 'Minéral sombre · menthe aurore',
      desktop_stable: 'Stable',
      ws_active: 'Espace de travail {n} — actif',
      app_ready: '{app} prêt',
      bp_meta_foundation: 'Fondation :',
      bp_meta_foundation_v: 'Dépendances verrouillées, générations atomiques, retour arrière',
      bp_meta_interface: 'Interface :',
      bp_meta_interface_v: 'Rust / WebAssembly (Living Canvas)',
      bp_meta_status: 'Statut :',
      bp_meta_status_v: 'Substrat, observation et diagnostic vérifiés · frontière d’action construite, exécuteur absent',
      bp_meta_updated: 'Mise à jour :',
      bp_meta_updated_v: 'Août 2026',
      btn_view_state: 'Voir l’état actuel',
      bp_stack1: '<strong>Système de base :</strong> Debian 13 est la cible de build, de vérification et de déploiement. Cybou est développé d’abord pour le serveur : un VPS, une VM ou un conteneur est l’environnement principal, pas un poste de travail.',
      bp_stack2: '<strong>Espace de travail Rust :</strong> un espace Cargo verrouillé portant le protocole et le codec fabric, les chemins d’exécution et de stockage, les tranches d’organes de remplacement, les contrats web, la passerelle en lecture seule et le frontend WebAssembly Living Canvas.',
      bp_stack3: '<strong>Livraison :</strong> la passerelle écoute la boucle locale, sert le frontend au hachage de contenu depuis sa propre origine, et expose des routes typées de session, d’instantané et de flux d’événements borné, sans route de mutation.',
      bp_stack4: '<strong>Session :</strong> une session Wayland optionnelle donne au frontend une surface unique via un compositeur minimal et un profil d’exécution éphémère. Elle est installée désactivée et n’est pas prouvée sur une machine avec siège ; le navigateur est la surface prise en charge.',
      bp_stack5: '<strong>Thème & esthétique :</strong> thème global Cybou Horizon, jetons de couleur clair/sombre, et une grammaire visuelle cohérente pour la connexion, les fenêtres et le fond d’écran.',
      bp_stack6: '<strong>Sorties de build :</strong> paquets Debian et unités utilisateur systemd, déployés sur un serveur par SSH et atteints via un navigateur.',
      bp_stack7: '<strong>Runtime Mind :</strong> des services utilisateur Rust activés par systemd et adressés via des interfaces D-Bus typées, dans un espace de travail où le code unsafe est interdit.',
      bp_stack8: '<strong>Mémoire canonique :</strong> un seul démon d’événements écrit dans le Journal ; le Journal v3 préserve la vérification des métadonnées causales tout en permettant un effacement sûr en cas de panne.',
      bp_stack9: '<strong>Continuité :</strong> identité, intentions, exécutions de cycle de vie, instantanés de santé et progression des consommateurs d’événements persistent sous schémas versionnés et écritures atomiques.',
      bp_stack10: '<strong>Modèle de panne :</strong> la perte d’un propriétaire optionnel ne dégrade que les capacités dépendantes ; la perte d’un propriétaire requis fait échouer les mutations sans inventer d’état.',
      bp_topology_title: 'Topologie des processus et de la propriété',
      th_milestone: 'Jalon',
      th_status: 'Statut',
      th_capability: 'Capacité',
      th_boundary: 'Frontière implémentée',
      th_evidence: 'Preuve principale',
      tag_done: 'FAIT',
      tag_inprogress: 'EN COURS',
      tag_planned: 'PLANIFIÉ',
      row_m0_cap: 'Base reproductible verte',
      row_m0_bound: 'Espace de travail Cargo verrouillé, contrôles de format, de licences, de métadonnées, de documentation et d’interface',
      row_m0_ev: 'Un script de contrôle sur toutes les vérifications, indiquant l’étape en échec',
      row_m14_cap: 'Mémoire acceptée et organes isolés',
      row_m14_bound: 'Un seul écrivain Event1, services isolés, proxy Presence distant',
      row_m14_ev: 'Tests de protocole, de processus, d’événements, d’API UI et de VM',
      row_m5_cap: 'Continuité et cycle de vie de consolidation',
      row_m5_bound: 'État d’exécution persistant, Lifecycle1, effets déterministes, reprise après redémarrage',
      row_m5_ev: 'Portes de continuité de cycle de vie et de panne à commit partiel',
      row_m6_cap: 'Santé, ordonnancement et reprise',
      row_m6_bound: 'Graphe de capacités, Health1, homéostasie v2, ordonnancement lié aux preuves, contrat UI dégradé',
      row_m6_ev: 'Frontière de reprise et matrice de pannes de processus',
      row_p67_cap: 'Orchestration bornée de Presence',
      row_p67_bound: 'Un budget monotone pour chaque lecture et mutation composée de Presence',
      row_p67_ev: 'RPC borné, tests de propriétaire suspendu et continuité KVM',
      row_m7_cap: 'Cognition ancrée et contexte gouverné',
      row_m7_bound: 'Perception, épistémique, effacement Journal v3, sensibilité, projection associative et livraison gouvernée',
      row_m7_ev: 'Portes ciblées d’unité, de processus, d’échelle, de rétention et de divulgation',
      row_body_cap: 'Observation, diagnostic et projection du Corps',
      row_body_bound: 'Télémétrie transitoire bornée qui n’entre jamais dans la biographie, constats qui portent les mesures dont ils viennent, projections issues de statistiques robustes, et ressources nommées déclarées par l’opérateur',
      row_body_ev: 'Portes du détecteur, des projections, de la liste de surveillance et du parcours de bout en bout',
      row_w01_cap: 'Fondation Rust et frontière web en lecture seule',
      row_w01_bound: 'Espace de travail Cargo verrouillé, crates de protocole, fabric, exécution et contrats web, passerelle locale avec session, instantané et flux d’événements reprenable',
      row_w01_ev: 'Tests natifs, lints stricts, build WebAssembly et portes du frontend de version',
      row_w2_cap: 'Une interface, servie à un navigateur',
      row_w2_bound: 'Frontend WebAssembly partagé derrière une passerelle en boucle locale, atteint via un proxy inverse ; la session de bureau à surface unique s’installe désactivée',
      row_w2_ev: 'Tests navigateur et portes de la passerelle ; la session de bureau n’est pas prouvée sur une machine avec siège',
      row_m8_cap: 'Faculté de langage optionnelle',
      row_m8_bound: 'Modèle remplaçable derrière des contrats typés de contexte et de proposition',
      row_m8_ev: 'Aucun modèle livré aujourd’hui',
      row_m913_cap: 'Apprentissage, action gouvernée, agents et sécurité',
      row_m913_bound: 'La frontière d’action est construite — propositions typées, critique, politique permanente, autorisation — et rien derrière elle ne peut exécuter. Artefacts appris, agents, outils et sécurité restent des frontières gouvernées séparées.',
      row_m913_ev: 'Les portes de proposition et d’autorisation passent ; aucun exécuteur n’existe, délibérément',
      bp_lic1: '<strong>Licence du code :</strong> licence MIT pour l’espace de travail Rust, les scripts de build et de déploiement, les ressources web et la logique centrale.',
      bp_lic2: '<strong>Ressources visuelles :</strong> Creative Commons Attribution-ShareAlike 4.0 pour les logos, fonds d’écran et thèmes.',
      bp_lic3: '<strong>Frontières de confiance :</strong> l’IPC local du même utilisateur n’est pas traité comme une frontière de sécurité complète ; toute action privilégiée future exigera une couche d’autorisation distincte.',
      bp_lic4: '<strong>Frontière d’interface :</strong> la passerelle refuse d’écouter ailleurs que sur la boucle locale, est en lecture seule par construction et applique des en-têtes no-store et de sécurité navigateur. Un déploiement joignable depuis l’extérieur place un proxy inverse devant elle et exige une connexion ; servir des inconnus est un mode défini par l’unité, jamais le défaut.',
      bp_lic5: '<strong>Confidentialité :</strong> les enveloppes d’événement portent des axes explicites de confidentialité et de sensibilité. Les charges sensibles sont scellées avec des clés par contribution, et l’effacement transitif sûr en cas de panne les détruit. Une sauvegarde qui a aussi capturé le magasin de clés sort de cette garantie, ce qui est testé et non supposé. L’expiration automatique et la réplication restent ouvertes.',
      bp_lic6: '<strong>Frontière cloud actuelle :</strong> le runtime et l’interface n’exigent aucun service d’IA hébergé ni clé d’API, et aucun modèle de langage n’est livré aujourd’hui.',
      bp_lic7: '<strong>Documents de sécurité :</strong> le dépôt maintient des modèles explicites de menace et de vie privée sous contrôle de version.',
      bp_sources_title: 'Sources canoniques et autorité des affirmations',
      bp_sources_lead: 'Ce livre blanc web est un résumé lisible. Les affirmations exactes d’implémentation sont régies par le dépôt :',
      bp_src1: '— frontière implémentée faisant autorité.',
      bp_src2: '— topologie, propriété et modèle à long terme.',
      bp_src3: '— preuves de processus, de redémarrage, de panne et KVM.',
      bp_src4: '— journal d’implémentation détaillé et travaux en cours.',
      bp_sec6_title: '6. Questions fréquentes',
      skip_link: 'Aller au contenu',
      metric_v_services: 'Services Mind typés',
      metric_v_tests: 'Une porte, aucun succès partiel',
      metric_v_crates: 'Espace Rust'
    },
    ru: {
      nav_experience: 'Интерфейс',
      nav_foundation: 'Основа',
      nav_progress: 'Прогресс',
      nav_design: 'Дизайн',
      nav_roadmap: 'Дорожная карта',
      nav_blueprint: 'Blueprint',
      nav_faq: 'FAQ',
      nav_partners: 'Партнёры',
      nav_github: 'GitHub',
      btn_view_blueprint: 'Blueprint',
      btn_back_home: '← На главную',
      part_badge: 'Партнёрство и поддержка',
      part_title: 'Партнёры, контакты и донаты',
      part_lead: 'Свяжитесь с автором Станиславом Савельевым, обсудите партнёрство по железу и софту или поддержите независимую открытую разработку Cybou.',
      part_contact_head: 'Официальный контакт',
      part_contact_text: 'Общие вопросы, пресса, сообщения об уязвимостях, сотрудничество дистрибутивов или поддержка оборудования:',
      part_donate_head: 'Поддержка и донаты',
      part_donate_text: 'Cybou — независимый открытый проект. Ваша поддержка идёт на воспроизводимые сборки, интерфейс на Rust/WebAssembly, разработку рантайма Mind и тесты восстановления после сбоев.',
      part_crypto_head: 'Криптодонаты',
      part_crypto_text: 'Поддержать Cybou напрямую в криптовалюте:',
      faq_h2_1: 'Частые',
      faq_h2_2: 'вопросы.',
      faq_q1: 'Что такое Cybou?',
      faq_a1: 'Cybou — экспериментальная операционная среда, спроектированная под агентов. Постоянный локальный рантайм Mind владеет долговременной памятью, идентичностью, обязательствами, жизненным циклом, здоровьем и свидетельствами, а модели, агенты, инструменты и интерфейсы остаются заменяемыми вокруг него.',
      faq_q2: 'Чем это отличается от чат-бота, прикрученного к Linux?',
      faq_a2: 'Модель — не Mind. Агент — не Mind. Протокол инструментов — не граница авторизации. Cybou делает память, идентичность, здоровье, жизненный цикл и восстановление явными системными сервисами с названными владельцами, поэтому никакая модель не может стать владельцем непрерывности или полномочий.',
      faq_q3: 'Нужны ли облачные сервисы и есть ли телеметрия?',
      faq_a3: 'Нет. Текущему рантайму и интерфейсу не нужны облачный аккаунт, API-ключ или хостинговый ИИ-сервис, а телеметрия не реализована вовсе. Удалённые модели могут появиться позже, но только за явной политикой контекста, чувствительности, исходящего трафика и стоимости.',
      faq_q4: 'На чём построен интерфейс?',
      faq_a4: 'Один фронтенд на Rust/WebAssembly под именем Living Canvas, обслуживаемый шлюзом на Rust, который отдаёт типизированные сессию, снапшот и возобновляемый поток событий — и ни одного маршрута мутации. Интерфейс — проекция Mind, а не второй владелец состояния, потому он и заменяем.',
      faq_q5: 'Что ещё не реализовано?',
      faq_a5: 'Cybou не поставляет языковую модель, рантайм агентов или воркеров, брокер моделей и инструментов, исполнитель привилегированных действий и автономный контур безопасности. Аутентификация при старте десктопа также остаётся открытой. Это запланированные границы, и они никогда не описываются как готовые.',
      faq_q6: 'Какие открытые лицензии применяются?',
      faq_a6: 'Код и большая часть документации Cybou — под лицензией MIT. Оригинальные визуальные ресурсы — под CC BY-SA 4.0. Репозиторий следует спецификации REUSE. Copyright (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Пред-релиз · сначала сервер · модель не поставляется',
      hero_h1_1: 'Linux, который понимает',
      hero_h1_2: 'и обслуживает себя сам.',
      hero_lead: 'Cybou — экспериментальная операционная среда, спроектированная под агентов, для сервера, VPS или контейнера: машины, которая работает без присмотра и от которой ждут, что она позаботится о себе. Она наблюдает собственное состояние, ставит диагноз и показывает измерения, из которых он получен. При отключённой сети и без загруженной модели.',
      hero_btn_explore: 'Собрать Cybou',
      hero_btn_blueprint: 'Читать технический blueprint',
      tag_unplugged: 'Работает без сети',
      tag_built_with: 'Rust · WebAssembly',
      tag_ai: 'Измерения, а не утверждения',
      tag_telemetry: 'Нам не сообщает ничего',
      desktop_welcome: 'Добро пожаловать в Cybou',
      desktop_sub: 'Тихая система, готовая, когда готовы вы.',
      launcher_search: 'Поиск приложений',
      app_files: 'Файлы',
      app_web: 'Веб',
      app_code: 'Код',
      app_settings: 'Настройки',
      gen_status: 'Система в порядке',
      visual_caption: 'Интерактивный концепт интерфейса — нажмите на знак Cybou',
      concept_badge: 'Концепт',
      exp_h2_1: 'Она наблюдает машину.',
      exp_h2_2: 'И объясняет себя.',
      exp_lead: 'Нагрузка, давление памяти и ввода-вывода, занятость файловой системы и инодов, открытые дескрипторы, упавшие юниты — и сертификаты, службы и резервные копии, которые вы объявили. То, что она из этого заключает, — гипотеза, несущая свои показания, а не факт. Спросите почему — она покажет числа, а не фразу, составленную моделью.',
      feat1_title: 'Living Canvas',
      feat1_desc: 'Единый фронтенд на Rust/WebAssembly: одна кодовая база, артефакт с хешем содержимого, отдаваемый с того же origin, что и его собственный API.',
      feat2_title: 'Граница только на чтение',
      feat2_desc: 'Шлюз слушает лишь локальную петлю, отвечает на типизированные запросы сессии, снапшота и событий в пределах заданного бюджета и не имеет ни одного маршрута мутации.',
      feat3_title: 'Спроектировано как система',
      feat3_desc: 'Цвет, геометрия, движение, экран входа и обои подчинены одной визуальной грамматике — и в тёмной, и в светлой теме.',
      found_h2_1: 'Она может предложить.',
      found_h2_2: 'Она не может действовать.',
      found_lead: 'Cybou формирует типизированные предложения по устранению, проверяет каждое против находки, которую оно берётся облегчить, и передаёт постоянной политике, которая не разрешает ничего, пока вы её не настроили. Ничто в этой сборке не может выполнить привилегированную операцию. Граница написана до исполнителя намеренно: в обратном порядке исполнитель приходит с решением действовать уже внутри себя.',
      p1_head: 'Предложено',
      p1_text: 'Типизированная операция из закрытого набора, а не текст для шелла.',
      p2_head: 'Раскритиковано',
      p2_text: 'Сверено с находкой, которую оно берётся облегчить.',
      p3_head: 'Разрешено',
      p3_text: 'Отказано, пока заданная вами политика этого не позволит.',
      gen_console_title: 'Состояния системы',
      gen147_desc: 'Текущее · основа интерфейса на Rust',
      gen_active: 'Активно',
      gen146_desc: 'Обновление безопасности · проверено',
      gen_yesterday: 'Вчера',
      gen145_desc: 'До настройки графики',
      gen_aug2: '2 авг.',
      prog_h2_1: 'Измеримое исполнение.',
      prog_h2_2: 'Проверенные этапы.',
      prog_lead: 'Фундамент несёт заземлённое восприятие, эпистемическую проекцию, устойчивое к сбоям стирание, управляемую доставку контекста, структурированный смысл, ограниченную телеметрию Тела и границу устранения, за которой нет исполнителя. Каждый слой должен был стать проверяемым прежде, чем следующему разрешили от него зависеть.',
      metric_gate_a: 'Изолированные сервисы Mind с явным типизированным владением через D-Bus',
      metric_tasks: 'Один скрипт ворот: формат, линт, нативные и браузерные тесты, валидаторы документов и слоёв, интеграция на живой шине',
      metric_contrast: 'Одно зафиксированное рабочее пространство Cargo, где unsafe запрещён повсюду',
      status_development: 'Реализованный рантайм',
      status_runtime: 'Проверенная база',
      status_verified: 'Миграция идёт',
      evidence_state: 'Текущее состояние',
      evidence_state_desc: 'Реализованное поведение и ограничения',
      evidence_tests: 'Доказательства тестами',
      evidence_tests_desc: 'Процессы, перезагрузка, сбои и KVM',
      evidence_source: 'Репозиторий',
      evidence_source_desc: 'Изучить код и историю разработки',
      design_sub: 'Визуальная система, построенная на глубине, воздухе и одной линии света.',
      mark_desc: 'Открытая дуга вокруг сфокусированного центра: система, у которой есть место расти.',
      type_head: 'Спокойно, точно, по-человечески.',
      type_desc: 'Системная гротескная типографика оставляет сайт независимым от внешних сервисов.',
      princ_h2_1: 'Заменяемое там, где можно.',
      princ_h2_2: 'Собственное там, где нужно.',
      pr1_head: 'Тихо по умолчанию',
      pr1_desc: 'Никакого анимированного маскота, светящегося ассистента и ленты, требующей внимания.',
      pr2_head: 'Поверхности заменяемы',
      pr2_desc: 'Интерфейсы, модели и инструменты можно менять, не трогая память, идентичность и политику.',
      pr3_head: 'Приватность по построению',
      pr3_desc: 'Ни телеметрии, ни обязательного аккаунта, ни загрузки ресурсов во время работы.',
      pr4_head: 'Построено этапами',
      pr4_desc: 'Каждый слой должен быть проверяем, прежде чем следующему разрешат на него опереться.',
      road_h2_1: 'Сначала тело.',
      road_h2_2: 'Потом разум.',
      rm1_title: 'Фундамент Mind',
      rm2_title: 'Rust-поверхность для браузера',
      rm3_title: 'Наблюдение и диагностика Тела',
      rm4_title: 'Управляемое действие и его результат',
      rm1_sub: 'Долговременная история · Идентичность · Стирание · Жизненный цикл',
      rm2_sub: 'Крейты протокола · Шлюз · Living Canvas',
      rm3_sub: 'Телеметрия · Находки · Проекции · Смысл',
      rm4_sub: 'Исполнитель · Независимый переосмотр · Агенты',
      rm_complete: 'Готово',
      rm_inprogress: 'В работе',
      bp_banner_kicker: 'Проект начинается здесь',
      bp_banner_h2_1: 'Настоящая операционная система,',
      bp_banner_h2_2: 'слой доверия за слоем.',
      bp_banner_desc: 'Blueprint описывает рантайм Mind и его типизированное владение, путь интерфейса на Rust/WebAssembly, слой наблюдения и диагностики, границу действия, проверенные ворота реализации и когнитивную дорожную карту.',
      bp_banner_btn: 'Читать технический документ',
      btn_top: 'Наверх',
      footer_sub: 'Среда для агентов · типизированный локальный Mind · интерфейс на Rust и WebAssembly.',
      bp_badge: 'Технический blueprint · архитектура и дорожная карта',
      bp_title: 'Cybou: управляемый когнитивный рантайм с заменяемым интерфейсом',
      bp_lead: 'Актуальный технический документ о рантайме Mind, модели владения и отказов, слое наблюдения и диагностики, границе действия, за которой нет исполнителя, проверенных воротах реализации и когнитивной дорожной карте.',
      btn_print_pdf: 'Экспорт в PDF / печать',
      bp_sec1_title: '1. Краткое резюме и видение',
      toc_vision: 'Видение',
      toc_architecture: 'Архитектура',
      toc_safety: 'Границы безопасности',
      toc_implementation: 'Реализация',
      toc_security: 'Безопасность и лицензии',
      toc_faq: 'FAQ',
      bp_sec1_p1: 'Cybou — экспериментальная операционная среда для агентов, состоящая из независимо проверяемых слоёв: воспроизводимой системной основы, Mind — локального типизированного рантайма для долговременной биографии, идентичности, обязательств, предсказаний, ограниченного внимания, здоровья и восстановления — и заменяемого интерфейса на Rust/WebAssembly. Это не чат-бот, и он не претендует на сознание.',
      bp_layer1_title: 'Слой 1: воспроизводимое тело',
      bp_layer1_desc: 'Пакеты Debian 13 и пользовательские юниты systemd, разворачиваемые на сервер по SSH, с явными воротами сборки и восстановления. Debian — среда сборки, проверки и развёртывания, а не хост, подменяющий собой другой.',
      bp_layer2_title: 'Слой 2: типизированный рантайм Mind',
      bp_layer2_desc: 'Изолированные пользовательские сервисы systemd общаются через типизированные контракты D-Bus. Event1 — каноническая граница Журнала; Presence — проекция, а не второй владелец. Языковых моделей и привилегированного исполнения нет.',
      bp_layer3_title: 'Слой 3: заменяемый интерфейс',
      bp_layer3_desc: 'Rust-воркспейс несёт протокол, кодек fabric, пути рантайма и хранилища и веб-контракты. Шлюз только на чтение на локальной петле проецирует Presence в типизированный JSON и ограниченный поток событий, а Living Canvas отображает это как WebAssembly в браузере или десктопной оболочке.',
      bp_sec2_title: '2. Архитектура и технический стек',
      bp_sec2_p1: 'Cybou разделяет Тело, Mind и Presence. Долговременное состояние принадлежит явным владельцам; любая поверхность — лишь кеш и граница представления. Межвладельческие команды ограничены и отказывают закрыто.',
      bp_stack_title: 'Основной стек системы',
      bp_sec3_title: '3. Когнитивные контракты и границы безопасности',
      bp_sec3_p1: 'Реализованный фундамент сохраняет проверяемость cognition и не превращает модель, UI или coordinator в неограниченного owner. Будущие функции обязаны сохранять контракты:',
      bp_ai_point1: '<strong>Ownership раньше intelligence:</strong> model ≠ identity, UI ≠ Mind, attention ≠ biography, proposal ≠ authorization.',
      bp_ai_point2: '<strong>Durability раньше visibility:</strong> состояние проецируется только после commit его owner; consolidation не переписывает историю.',
      bp_ai_point3: '<strong>Ограниченная деградация:</strong> Health1 публикует deficits и recovery; составные чтения и mutations используют один монотонный deadline.',
      bp_ai_point4: '<strong>Никакой скрытой agency:</strong> интерфейс может только читать; язык M8 необязателен, обучение M9, действия M10 и агенты M11 остаются отдельными управляемыми границами.',
      bp_sec4_title: '4. Прогресс разработки и матрица задач',
      bp_sec4_p1: 'Ход разработки строго фиксируется по этапам и проверяется авто-тестами.',
      bp_sec5_title: '5. Безопасность, приватность и лицензии',
      bp_sec5_p1: 'Cybou — свободное ПО, созданное для цифрового суверенитета:',
      nav_partners_footer: 'Партнёры и донаты',
      kicker_interface: '01 · Интерфейс',
      kicker_foundation: '02 · Основа',
      kicker_progress: '03 · Реальный прогресс',
      kicker_design: '04 · Визуальный язык',
      kicker_principles: '05 · Принципы',
      kicker_roadmap: '06 · Дорожная карта',
      kicker_faq: '07 · Вопросы и архитектура',
      label_symbol: 'Символ',
      label_palette: 'Палитра',
      label_typography: 'Типографика',
      palette_name: 'Минеральный тёмный · мятная аврора',
      desktop_stable: 'Стабильно',
      ws_active: 'Рабочий стол {n} — активен',
      app_ready: '{app} готово',
      bp_meta_foundation: 'Основа:',
      bp_meta_foundation_v: 'Зафиксированные зависимости, атомарные поколения, откат',
      bp_meta_interface: 'Интерфейс:',
      bp_meta_interface_v: 'Rust / WebAssembly (Living Canvas)',
      bp_meta_status: 'Статус:',
      bp_meta_status_v: 'Фундамент, наблюдение и диагностика проверены · граница действия построена, исполнителя нет',
      bp_meta_updated: 'Обновлено:',
      bp_meta_updated_v: 'Август 2026',
      btn_view_state: 'Открыть текущее состояние',
      bp_stack1: '<strong>Базовая система:</strong> Debian 13 — цель сборки, проверки и развёртывания. Cybou разрабатывается прежде всего под сервер: VPS, виртуальная машина или контейнер — основная среда, а не рабочая станция.',
      bp_stack2: '<strong>Rust-воркспейс:</strong> один зафиксированный воркспейс Cargo с протоколом и кодеком fabric, путями рантайма и хранилища, замещающими срезами органов, веб-контрактами, шлюзом только на чтение и WebAssembly-фронтендом Living Canvas.',
      bp_stack3: '<strong>Доставка:</strong> шлюз слушает локальную петлю, отдаёт фронтенд с хешем содержимого со своего же origin и предоставляет типизированные маршруты сессии, снапшота и ограниченного потока событий — без единого маршрута мутации.',
      bp_stack4: '<strong>Сессия:</strong> опциональная Wayland-сессия даёт фронтенду единственную поверхность через минимальный композитор и эфемерный профиль исполнения. Она ставится отключённой и не проверена на машине с seat; поддерживаемая поверхность — браузер.',
      bp_stack5: '<strong>Тема и эстетика:</strong> глобальная тема Cybou Horizon, цветовые токены светлой и тёмной схем и единая визуальная грамматика для входа, окон и обоев.',
      bp_stack6: '<strong>Результат сборки:</strong> пакеты Debian и пользовательские юниты systemd, разворачиваемые на сервер по SSH и доступные через браузер.',
      bp_stack7: '<strong>Рантайм Mind:</strong> пользовательские сервисы на Rust, запускаемые systemd и адресуемые через типизированные интерфейсы D-Bus, в одном рабочем пространстве, где unsafe запрещён.',
      bp_stack8: '<strong>Каноническая память:</strong> единственный демон событий — единственный писатель Журнала; Журнал v3 сохраняет проверяемость причинных метаданных и допускает устойчивое к сбоям стирание полезной нагрузки.',
      bp_stack9: '<strong>Непрерывность:</strong> идентичность, намерения, запуски жизненного цикла, снимки здоровья и прогресс потребителей событий сохраняются по версионированным схемам и атомарным записям.',
      bp_stack10: '<strong>Модель отказов:</strong> потеря необязательного владельца деградирует только зависимые возможности; потеря обязательного владельца закрывает мутации, не выдумывая состояние.',
      bp_topology_title: 'Топология процессов и владения',
      th_milestone: 'Этап',
      th_status: 'Статус',
      th_capability: 'Возможность',
      th_boundary: 'Реализованная граница',
      th_evidence: 'Основное доказательство',
      tag_done: 'ГОТОВО',
      tag_inprogress: 'В РАБОТЕ',
      tag_planned: 'ПЛАН',
      row_m0_cap: 'Зелёная воспроизводимая база',
      row_m0_bound: 'Зафиксированное рабочее пространство Cargo, проверки формата, лицензий, метаданных, документов и интерфейса',
      row_m0_ev: 'Один скрипт ворот по всем проверкам, называющий упавший шаг',
      row_m14_cap: 'Принятая память и изолированные органы',
      row_m14_bound: 'Единственный писатель Event1, изолированные сервисы, удалённый прокси Presence',
      row_m14_ev: 'Тесты протокола, процессов, событий, UI-API и VM',
      row_m5_cap: 'Непрерывность и жизненный цикл консолидации',
      row_m5_bound: 'Персистентное состояние запуска, Lifecycle1, детерминированные эффекты, восстановление после перезапуска и перезагрузки',
      row_m5_ev: 'Гейты непрерывности жизненного цикла и сбоя при частичном коммите',
      row_m6_cap: 'Здоровье, планирование и восстановление',
      row_m6_bound: 'Граф возможностей, Health1, гомеостаз v2, планирование по свидетельствам, контракт деградированного UI',
      row_m6_ev: 'Граница восстановления и матрица сбоев процессов',
      row_p67_cap: 'Ограниченная оркестрация Presence',
      row_p67_bound: 'Один монотонный бюджет на каждое составное чтение и мутацию Presence',
      row_p67_ev: 'Ограниченный RPC, тесты приостановленного владельца и непрерывность в KVM',
      row_m7_cap: 'Заземлённое познание и управляемый контекст',
      row_m7_bound: 'Восприятие, эпистемика, стирание в Журнале v3, чувствительность, ассоциативная проекция и управляемая доставка',
      row_m7_ev: 'Точечные гейты модулей, процессов, масштаба, хранения и раскрытия',
      row_body_cap: 'Наблюдение, диагностика и проекция Тела',
      row_body_bound: 'Ограниченная транзитная телеметрия, которая никогда не попадает в биографию, находки, несущие свои показания, проекции из робастной статистики и именованные ресурсы, объявленные оператором',
      row_body_ev: 'Ворота детектора, проекций, списка наблюдения и сквозного прохода',
      row_w01_cap: 'Основа на Rust и веб-граница только на чтение',
      row_w01_bound: 'Зафиксированный воркспейс Cargo, крейты протокола, fabric, рантайма и веб-контрактов, локальный шлюз с типизированными сессией, снапшотом и возобновляемым потоком событий',
      row_w01_ev: 'Нативные тесты, строгие линты, сборка WebAssembly и гейты релизного фронтенда',
      row_w2_cap: 'Один интерфейс, отдаваемый браузеру',
      row_w2_bound: 'Общий WebAssembly-фронтенд за шлюзом на локальной петле, доступный через обратный прокси; десктопная сессия с единственной поверхностью ставится отключённой',
      row_w2_ev: 'Браузерные тесты и ворота шлюза; десктопная сессия не проверена на машине с seat',
      row_m8_cap: 'Опциональная языковая способность',
      row_m8_bound: 'Заменяемая модель за типизированными контрактами контекста и предложений',
      row_m8_ev: 'Модель сегодня не поставляется',
      row_m913_cap: 'Обучение, управляемое действие, агенты и безопасность',
      row_m913_bound: 'Граница действия построена — типизированные предложения, критика, постоянная политика, разрешение — и ничто за ней не может выполнять. Обученные артефакты, агенты, инструменты и безопасность остаются отдельными управляемыми границами.',
      row_m913_ev: 'Ворота предложения и разрешения проходят; исполнителя нет, намеренно',
      bp_lic1: '<strong>Лицензия кода:</strong> MIT для рабочего пространства Rust, скриптов сборки и развёртывания, веб-ресурсов и основной логики.',
      bp_lic2: '<strong>Визуальные ресурсы:</strong> Creative Commons Attribution-ShareAlike 4.0 для логотипов, обоев и тем.',
      bp_lic3: '<strong>Границы доверия:</strong> локальный IPC того же пользователя не считается полной границей безопасности возможностей; будущее привилегированное действие потребует отдельного слоя авторизации.',
      bp_lic4: '<strong>Граница интерфейса:</strong> шлюз отказывается слушать что-либо кроме локальной петли, работает только на чтение по построению и выставляет no-store и заголовки браузерной безопасности. Развёртывание, доступное извне машины, ставит перед ним обратный прокси и требует входа; обслуживать посторонних — режим, который задаёт юнит, а не то, что происходит по умолчанию.',
      bp_lic5: '<strong>Приватность:</strong> конверты событий несут явные оси приватности и чувствительности. Чувствительные полезные нагрузки запечатываются ключами на вклад, а устойчивое к сбоям транзитивное стирание их уничтожает. Резервная копия, забравшая и хранилище ключей, находится вне этой гарантии — это проверено тестом, а не предположено. Автоматическое истечение и репликация остаются открытыми.',
      bp_lic6: '<strong>Текущая граница облака:</strong> рантайму и интерфейсу не нужны хостинговый ИИ-сервис или API-ключ, и языковая модель сегодня не поставляется.',
      bp_lic7: '<strong>Документы безопасности:</strong> репозиторий поддерживает явные модели угроз и приватности под контролем версий.',
      bp_sources_title: 'Канонические источники и авторитет утверждений',
      bp_sources_lead: 'Этот веб-документ — читаемое резюме. Точные утверждения о реализации определяются репозиторием:',
      bp_src1: '— авторитетная реализованная граница.',
      bp_src2: '— топология, владение и долгосрочная модель.',
      bp_src3: '— доказательства процессов, перезагрузки, сбоев и KVM.',
      bp_src4: '— подробный журнал реализации и текущая работа.',
      bp_sec6_title: '6. Частые вопросы',
      skip_link: 'Перейти к содержимому',
      metric_v_services: 'Типизированные сервисы Mind',
      metric_v_tests: 'Одни ворота, без частичного прохода',
      metric_v_crates: 'Rust-воркспейс'
    }
  };

  let currentLang = localStorage.getItem('cybou_lang') || 'en';

  // Dictionary strings are authored in this file, never user input. Only the inline emphasis
  // markup below is honoured; everything else is escaped, so a stray '<' can never become an element.
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

    // A language variant that canonicalises to another URL is dropped from the index,
    // so each one points at itself: the bare page for English, ?lang= for the rest.
    const canonical = document.querySelector('link[rel="canonical"]');
    if (canonical) {
      const base = canonical.href.split('?')[0];
      const self = lang === 'en' ? base : `${base}?lang=${lang}`;
      canonical.href = self;
      document.querySelector('meta[property="og:url"]')?.setAttribute('content', self);
    }
  };

  // ?lang= is the canonical form, because a query string makes a distinct URL that
  // search engines can index per language; #lang= stays honoured for older links.
  const requested = new URLSearchParams(window.location.search).get('lang')
    || (window.location.hash.match(/lang=(en|fr|ru)/) || [])[1];
  if (requested && translations[requested]) {
    currentLang = requested;
  }

  setLanguage(currentLang);

  // Dropdown Language Switcher Listeners
  document.querySelectorAll('[data-lang-dropdown]').forEach((container) => {
    const trigger = container.querySelector('[data-lang-trigger]');
    const menu = container.querySelector('[data-lang-menu]');

    trigger?.addEventListener('click', (e) => {
      e.stopPropagation();
      const open = !menu?.classList.contains('open');
      document.querySelectorAll('[data-lang-menu]').forEach((m) => m.classList.remove('open'));
      document.querySelectorAll('[data-lang-trigger]').forEach((t) => t.classList.remove('open'));
      menu?.classList.toggle('open', open);
      trigger.classList.toggle('open', open);
      trigger.setAttribute('aria-expanded', String(open));
    });
  });

  document.querySelectorAll('[data-lang]').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const lang = btn.getAttribute('data-lang');
      setLanguage(lang);
      document.querySelectorAll('[data-lang-menu]').forEach((m) => m.classList.remove('open'));
      document.querySelectorAll('[data-lang-trigger]').forEach((t) => t.classList.remove('open'));
    });
  });

  document.addEventListener('click', () => {
    document.querySelectorAll('[data-lang-menu]').forEach((m) => m.classList.remove('open'));
    document.querySelectorAll('[data-lang-trigger]').forEach((t) => t.classList.remove('open'));
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

  const revealNodes = document.querySelectorAll('.reveal');
  if ('IntersectionObserver' in window && !window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.classList.add('visible');
          observer.unobserve(entry.target);
        }
      });
    }, { threshold: 0.14, rootMargin: '0px 0px -40px' });
    revealNodes.forEach((node) => observer.observe(node));
  } else {
    revealNodes.forEach((node) => node.classList.add('visible'));
  }

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
