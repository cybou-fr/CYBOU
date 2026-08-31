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
      btn_top: 'Back to top',
      btn_view_demo: 'Request Demo',
      concept_badge: 'Living Canvas Preview',
      demo_banner_btn: 'Request Live Demo',
      demo_banner_desc: 'Schedule a personalized demonstration of the 14-daemon Mind runtime, kernel-enforced agent capsules, Leptos/WASM spatial desktop, and the cryptographic Event1 ledger on Debian 13.',
      demo_banner_h2_1: 'Request a Private Live Demo &',
      demo_banner_h2_2: 'Pilot Deployment Walkthrough.',
      demo_banner_kicker: 'Experience Sovereign AI',
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
      hero_btn_demo: 'Request Live Demo',
      hero_btn_explore: 'Explore Code & Architecture',
      hero_eyebrow: 'Developer Preview · 100% Rust & WebAssembly · Debian 13',
      hero_h1_1: 'The Sovereign, Agent-Native',
      hero_h1_2: 'Cognitive Operating Environment.',
      hero_lead: 'Stop wrapping probabilistic LLMs in vulnerable shell scripts. Cybou is the open-source, agent-native cognitive substrate built entirely in pure Rust for Debian 13 Linux: a deterministic local control plane (Mind), kernel-enforced agent capsules (Agent1), two-phase action governance, and a tamper-evident cryptographic event ledger. 100% local-sufficient with zero cloud dependencies.',
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
      nav_demo: 'Demo',
      nav_design: 'Spatial Canvas',
      nav_experience: 'Interface',
      nav_faq: 'FAQ',
      nav_foundation: 'Governance',
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
      part_badge: 'Partnership & Demo Hub',
      part_contact_head: 'Official Contact & Collaboration',
      part_contact_text: 'For enterprise partnerships, security disclosures, hardware enablement, and open-source Linux distribution collaboration:',
      part_crypto_head: 'Cryptocurrency Donations',
      part_crypto_text: 'Direct sponsorship addresses for supporting Cybou development:',
      skip_link: 'Skip to content',
      part_demo_btn: 'Email for Demo (info@cybou.fr)',
      part_demo_head: 'Request Live Demo & Pilot Access',
      part_demo_text: 'We provide live technical walkthroughs for engineering teams, sovereign cloud operators, and researchers. Email us directly with your target environment or use cases to schedule a dedicated session:',
      part_donate_head: 'Support & Sponsorship',
      part_donate_text: 'Cybou is an independent open-source project. Your financial support directly funds deterministic control plane development, reproducible builds, and rigorous security testing.',
      part_lead: 'Schedule a private demo walkthrough of Cybou, explore pilot integration on your infrastructure, or support independent sovereign software engineering.',
      part_title: 'Live Demo, Partners & Sponsorship',
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
      status_development: 'Active Substrate',
      status_runtime: 'Verified Baseline',
      status_verified: 'Operational',
      tag_ai: 'Epistemic Truth, Not Hallucinations',
      tag_built_with: '100% Rust · WebAssembly',
      tag_telemetry: 'Zero Remote Telemetry',
      tag_unplugged: '100% Local-Sufficient',
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
      btn_top: 'Haut de page',
      btn_view_demo: 'Demander une Démo',
      concept_badge: 'Aperçu Living Canvas',
      demo_banner_btn: 'Demander une Démo',
      demo_banner_desc: 'Planifiez une démonstration technique personnalisée du runtime Mind à 14 démons, des capsules d\'agents isolées par le noyau, du bureau spatial Leptos/WASM et du registre cryptographique Event1.',
      demo_banner_h2_1: 'Demandez une Démo en Direct &',
      demo_banner_h2_2: 'un Accès Pilote Dédié.',
      demo_banner_kicker: 'Découvrez l\'IA Souveraine',
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
      hero_btn_demo: 'Demander une Démo en direct',
      hero_btn_explore: 'Explorer le code & l\'architecture',
      hero_eyebrow: 'Aperçu développeur · 100% Rust & WebAssembly · Debian 13',
      hero_h1_1: 'L\'Environnement Opérationnel',
      hero_h1_2: 'Souverain et Orienté Agents.',
      hero_lead: 'Cessez d\'envelopper des LLM probabilistes dans des scripts shell vulnérables. Cybou est le substrat cognitif orienté agents conçu entièrement en Rust pour Debian 13 Linux : plan de contrôle local déterministe (Mind), capsules d\'agents isolées par le noyau (Agent1), gouvernance d\'actions à deux phases et registre cryptographique inviolable. 100% autosuffisant en local avec zéro dépendance cloud.',
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
      nav_demo: 'Démo',
      nav_design: 'Canevas Spatial',
      nav_experience: 'Interface',
      nav_faq: 'FAQ',
      nav_foundation: 'Gouvernance',
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
      part_badge: 'Espace Démo & Partenariats',
      part_contact_head: 'Contact Officiel & Collaboration',
      part_contact_text: 'Pour les partenariats d\'entreprise, la sécurité, l\'adaptation matérielle et la collaboration avec les distributions Linux :',
      part_crypto_head: 'Dons en Cryptomonnaies',
      part_crypto_text: 'Adresses directes pour soutenir le développement de Cybou :',
      skip_link: 'Passer au contenu',
      part_demo_btn: 'Écrire pour une démo (info@cybou.fr)',
      part_demo_head: 'Demander une Démo en Direct & un Accès Pilote',
      part_demo_text: 'Nous proposons des présentations techniques dédiées aux équipes d\'ingénierie, aux opérateurs de clouds souverains et aux chercheurs. Écrivez-nous pour planifier votre session :',
      part_donate_head: 'Soutien & Financement',
      part_donate_text: 'Cybou est un projet open-source indépendant. Votre soutien financier finance directement le développement du plan de contrôle déterministe, les builds reproductibles et les tests de sécurité.',
      part_lead: 'Planifiez une démonstration personnalisée de Cybou, évaluez son déploiement pilote sur votre infrastructure, ou soutenez l\'ingénierie logicielle souveraine indépendante.',
      part_title: 'Démo en Direct, Partenaires & Parrainage',
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
      status_development: 'Substrat Actif',
      status_runtime: 'Base Vérifiée',
      status_verified: 'Opérationnel',
      tag_ai: 'Vérité épistémique, zéro hallucination',
      tag_built_with: '100% Rust · WebAssembly',
      tag_telemetry: 'Zéro télémétrie distante',
      tag_unplugged: '100% Autosuffisant en local',
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
      btn_top: 'Наверх',
      btn_view_demo: 'Запросить демо',
      concept_badge: 'Превью Living Canvas',
      demo_banner_btn: 'Запросить демо',
      demo_banner_desc: 'Запланируйте персональную демонстрацию управляющего контура Mind (14 демонов), песочниц агентов, пространственного десктопа Leptos/WASM и криптографического реестра Event1 на Debian 13.',
      demo_banner_h2_1: 'Запросите живую демонстрацию и',
      demo_banner_h2_2: 'пилотный доступ к системе.',
      demo_banner_kicker: 'Оцените Суверенный ИИ',
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
      hero_btn_demo: 'Запросить живое демо',
      hero_btn_explore: 'Изучить код и архитектуру',
      hero_eyebrow: 'Developer Preview · 100% Rust и WebAssembly · Debian 13',
      hero_h1_1: 'Суверенная, Агентная',
      hero_h1_2: 'Когнитивная Операционная Среда.',
      hero_lead: 'Хватит оборачивать вероятностные нейросети в уязвимые shell-скрипты. CYBOU — это открытая, агентно-ориентированная операционная среда, созданная полностью на чистом Rust для Debian 13 Linux: детерминированный локальный управляющий контур (Mind), изолированные ядром капсулы агентов (Agent1), двухфазный контроль действий и криптографический журнал событий. 100% локальная автономность с нулевой зависимостью от облаков.',
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
      nav_demo: 'Демо',
      nav_design: 'Холст',
      nav_experience: 'Интерфейс',
      nav_faq: 'FAQ',
      nav_foundation: 'Управление',
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
      part_badge: 'Демо и Партнёрство',
      part_contact_head: 'Официальные контакты и сотрудничество',
      part_contact_text: 'По вопросам корпоративного партнёрства, безопасности, адаптации под аппаратные платформы и сотрудничества с дистрибутивами Linux:',
      part_crypto_head: 'Криптовалютные пожертвования',
      part_crypto_text: 'Прямые адреса для поддержки разработки CYBOU:',
      skip_link: 'Перейти к содержимому',
      part_demo_btn: 'Написать по поводу демо (info@cybou.fr)',
      part_demo_head: 'Запрос живого демо и пилотного доступа',
      part_demo_text: 'Мы проводим персональные технические демонстрации для инженерных команд, операторов суверенных облаков и исследователей. Напишите нам на почту, чтобы согласовать удобное время:',
      part_donate_head: 'Поддержка и спонсорство',
      part_donate_text: 'CYBOU — независимый открытый проект. Ваша финансовая поддержка напрямую финансирует разработку детерминированного ядра, воспроизводимые сборки и тестирование безопасности.',
      part_lead: 'Запланируйте персональное демо CYBOU, протестируйте пилотное развёртывание на своей инфраструктуре или поддержите независимую разработку суверенного ПО.',
      part_title: 'Живое демо, Партнёры и Поддержка',
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
      status_development: 'Активный субстрат',
      status_runtime: 'Проверенный базис',
      status_verified: 'Работоспособен',
      tag_ai: 'Эмпирическая правда, а не галлюцинации',
      tag_built_with: '100% Rust · WebAssembly',
      tag_telemetry: 'Нулевая внешняя телеметрия',
      tag_unplugged: '100% Локальная автономность',
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
