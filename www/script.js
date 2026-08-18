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
      part_crypto_text: 'Support Stanislav Saveliev directly via cryptocurrency:',
      faq_h2_1: 'Frequently asked',
      faq_h2_2: 'questions.',
      faq_q1: 'What is Cybou?',
      faq_a1: 'Cybou is an experimental agent-native operating environment. A persistent local runtime called Mind owns durable memory, identity, commitments, lifecycle, health, and evidence, while models, agents, tools, and user interfaces stay replaceable around it.',
      faq_q2: 'How is that different from adding a chatbot to Linux?',
      faq_a2: 'A model is not Mind. An agent is not Mind. A tool protocol is not an authorization boundary. Cybou makes memory, identity, health, lifecycle, and recovery explicit system services with named owners, so no model can become the owner of continuity or authority.',
      faq_q3: 'Does Cybou require cloud services or send telemetry?',
      faq_a3: 'No. The current runtime and interface need no cloud account, API key, or hosted AI service, and Cybou implements no telemetry. Remote models may become available later, but only behind explicit context, sensitivity, egress, and cost policy.',
      faq_q4: 'What is the interface built with?',
      faq_a4: 'One Rust/WebAssembly frontend called Living Canvas serves both an ordinary browser and a lightweight desktop web shell. It reads through a read-only Rust gateway bound to loopback that exposes typed session, snapshot, and event streams and has no mutation route. The earlier KDE/Qt shell is migration-era code, not the target.',
      faq_q5: 'What is not implemented yet?',
      faq_a5: 'Cybou ships no language model, no agent or worker runtime, no model or tool broker, no privileged action executor, and no autonomous security control plane. Desktop bootstrap authentication is also still open. These are planned boundaries and are never described as done.',
      faq_q6: 'What open-source licenses apply?',
      faq_a6: 'Cybou code and Nix expressions are licensed under MIT. Original visual assets (wallpapers, desktop themes, SVGs) are licensed under CC BY-SA 4.0. Copyright (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Pre-release · Rust/WebAssembly migration in progress',
      hero_h1_1: 'A system that remembers.',
      hero_h1_2: 'A mind that stays in charge.',
      hero_lead: 'Cybou is an experimental agent-native operating environment. Twelve local services give it durable memory, identity, health, and recovery — and one Rust/WebAssembly interface renders it in a browser or a desktop shell. No shipped model, no required cloud.',
      hero_btn_explore: 'Build Cybou',
      hero_btn_blueprint: 'Read Technical Blueprint',
      tag_nixos: 'Reproducible foundation',
      tag_plasma: 'Rust · WebAssembly',
      tag_ai: 'Typed local runtime',
      tag_telemetry: 'Zero Telemetry',
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
      exp_h2_1: 'One interface.',
      exp_h2_2: 'Two surfaces.',
      exp_lead: 'The same Rust/WebAssembly frontend runs in an ordinary browser and in a minimal desktop shell that owns a single Wayland surface. The interface is a projection of Mind, never a second owner of state — which is why it can be replaced without touching memory, identity, or policy.',
      feat1_title: 'Living Canvas',
      feat1_desc: 'One Rust/WebAssembly frontend, compiled from a single source, delivered as a content-hashed artifact from the same origin as its own API.',
      feat2_title: 'A read-only boundary',
      feat2_desc: 'The gateway binds to loopback, answers typed session, snapshot, and event requests under a bounded budget, and exposes no mutation route at all.',
      feat3_title: 'Designed as a system',
      feat3_desc: 'Color, geometry, motion, login, and wallpaper share one coherent visual grammar in both dark and light.',
      found_h2_1: 'Declarative by nature.',
      found_h2_2: 'Reversible by design.',
      found_lead: 'Underneath, Cybou inherits reproducible configuration and generation rollback. Updates become explicit system states — understandable, testable and recoverable. Nothing becomes visible before its owner has durably committed it.',
      p1_head: 'Reproducible',
      p1_text: 'Build the same system from a locked configuration.',
      p2_head: 'Recoverable',
      p2_text: 'Return to a previous generation when an update fails.',
      p3_head: 'Inspectable',
      p3_text: 'Know what changed before trusting a new state.',
      gen_console_title: 'System states',
      gen147_desc: 'Current · Rust interface foundation',
      gen_active: 'Active',
      gen146_desc: 'Security update · verified',
      gen_yesterday: 'Yesterday',
      gen145_desc: 'Before graphics configuration',
      gen_aug2: 'Aug 2',
      prog_h2_1: 'Measured execution.',
      prog_h2_2: 'Verified milestones.',
      prog_lead: 'The verified M1–M6 substrate now carries grounded perception, epistemic projection, crash-safe erasure, and governed context delivery — while the interface layer migrates to Rust and WebAssembly through additive, separately gated steps.',
      metric_gate_a: 'Twelve isolated Mind services with explicit typed ownership',
      metric_tasks: 'Thirty-seven CTest suites plus reproducible repository policy gates',
      metric_contrast: 'Five Rust crates: protocol, fabric, web contracts, gateway, Living Canvas',
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
      rm1_title: 'Reproducible foundation',
      rm2_title: 'Mind substrate',
      rm3_title: 'Web-first Rust surface',
      rm4_title: 'Grounded cognition and governed action',
      rm1_sub: 'Locked builds · Horizon visual system · ISO',
      rm2_sub: 'M1–M6 · P6.8 · Lifecycle and recovery',
      rm3_sub: 'Protocol crates · Gateway · Living Canvas',
      rm4_sub: 'Perception · Language · Agents · Security',
      rm_complete: 'Complete',
      rm_inprogress: 'In progress',
      bp_banner_kicker: 'The project starts here',
      bp_banner_h2_1: 'A real operating system,',
      bp_banner_h2_2: 'built one trusted layer at a time.',
      bp_banner_desc: 'The blueprint defines the reproducible foundation, the twelve-service Mind runtime, the Rust/WebAssembly interface path, the verified implementation gates, and the cognitive roadmap.',
      bp_banner_btn: 'Read Technical Whitepaper',
      btn_top: 'Back to top',
      footer_sub: 'Agent-native environment · Typed local Mind · Rust and WebAssembly interface.',
      bp_badge: 'Technical Blueprint · Current Architecture & Roadmap',
      bp_title: 'Cybou: A Governed Cognitive Runtime with a Replaceable Interface',
      bp_lead: 'The current whitepaper for Cybou’s twelve-service Mind runtime, its ownership and failure model, the verified M1–M6 substrate, the additive Rust/WebAssembly interface migration, and the M7–M13 roadmap.',
      btn_print_pdf: 'Export to PDF / Print',
      btn_explore_landing: 'Explore Web Landing',
      bp_sec1_title: '1. Executive Summary & Core Vision',
      toc_vision: 'Vision',
      toc_architecture: 'Architecture',
      toc_safety: 'Safety boundaries',
      toc_implementation: 'Implementation',
      toc_security: 'Security & licensing',
      toc_faq: 'FAQ',
      bp_sec1_p1: 'Cybou is an experimental agent-native operating environment with independently testable layers: a reproducible system foundation, Mind — a local typed runtime for durable biography, identity, commitments, prediction, bounded attention, health and recovery — and a replaceable Rust/WebAssembly interface. It is not a chatbot and makes no claim of consciousness.',
      bp_layer1_title: 'Layer 1: Reproducible Body',
      bp_layer1_desc: 'Locked Flakes, atomic generations, VM/ISO/Hyper-V outputs, and explicit build and recovery gates. Debian 13 is the active build and verification environment for every Linux gate.',
      bp_layer2_title: 'Layer 2: Typed Mind Runtime',
      bp_layer2_desc: 'Twelve isolated systemd user services communicate through typed D-Bus contracts. Event1 is the canonical Journal boundary; Presence is a projection, not a second owner. Language models and privileged execution remain absent.',
      bp_layer3_title: 'Layer 3: Replaceable Interface',
      bp_layer3_desc: 'A Rust workspace carries the protocol, fabric codec, and web contracts. A read-only gateway on loopback projects Presence into typed JSON and a bounded event stream, and Living Canvas renders it as WebAssembly in a browser or desktop shell.',
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
      bp_sec5_p1: 'Cybou is free software built for sovereignty and privacy:'
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
      part_crypto_text: 'Soutenez directement Stanislav Saveliev en cryptomonnaie :',
      faq_h2_1: 'Questions',
      faq_h2_2: 'fréquentes.',
      faq_q1: 'Qu’est-ce que Cybou ?',
      faq_a1: 'Cybou est un environnement d’exploitation expérimental conçu pour les agents. Un runtime local persistant nommé Mind détient la mémoire durable, l’identité, les engagements, le cycle de vie, la santé et les preuves, tandis que les modèles, agents, outils et interfaces restent remplaçables autour de lui.',
      faq_q2: 'En quoi est-ce différent d’un chatbot ajouté à Linux ?',
      faq_a2: 'Un modèle n’est pas Mind. Un agent n’est pas Mind. Un protocole d’outils n’est pas une frontière d’autorisation. Cybou fait de la mémoire, de l’identité, de la santé, du cycle de vie et de la reprise des services système explicites avec des propriétaires nommés : aucun modèle ne peut devenir propriétaire de la continuité ni de l’autorité.',
      faq_q3: 'Cybou exige-t-il le cloud ou envoie-t-il de la télémétrie ?',
      faq_a3: 'Non. Le runtime et l’interface actuels ne demandent aucun compte cloud, clé d’API ni service d’IA hébergé, et Cybou n’implémente aucune télémétrie. Des modèles distants pourront exister plus tard, mais uniquement derrière une politique explicite de contexte, de sensibilité, de sortie réseau et de coût.',
      faq_q4: 'Avec quoi l’interface est-elle construite ?',
      faq_a4: 'Un unique frontend Rust/WebAssembly nommé Living Canvas sert à la fois un navigateur ordinaire et un shell de bureau léger. Il lit à travers une passerelle Rust en lecture seule liée à la boucle locale, qui expose des flux typés de session, d’instantané et d’événements, sans aucune route de mutation. L’ancien shell KDE/Qt est du code de migration, pas la cible.',
      faq_q5: 'Qu’est-ce qui n’est pas encore implémenté ?',
      faq_a5: 'Cybou ne livre aucun modèle de langage, aucun runtime d’agents ou de workers, aucun courtier de modèles ou d’outils, aucun exécuteur d’actions privilégiées et aucun plan de contrôle de sécurité autonome. L’authentification au démarrage du bureau reste également ouverte. Ce sont des frontières planifiées, jamais décrites comme achevées.',
      faq_q6: 'Quelles licences open source s’appliquent ?',
      faq_a6: 'Le code et les expressions Nix de Cybou sont sous licence MIT. Les ressources visuelles originales (fonds d’écran, thèmes, SVG) sont sous CC BY-SA 4.0. Copyright (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Pré-version · migration Rust/WebAssembly en cours',
      hero_h1_1: 'Un système qui se souvient.',
      hero_h1_2: 'Un esprit qui garde la main.',
      hero_lead: 'Cybou est un environnement d’exploitation expérimental conçu pour les agents. Douze services locaux lui donnent mémoire durable, identité, santé et reprise — et une seule interface Rust/WebAssembly l’affiche dans un navigateur ou un shell de bureau. Aucun modèle livré, aucun cloud requis.',
      hero_btn_explore: 'Compiler Cybou',
      hero_btn_blueprint: 'Lire le blueprint technique',
      tag_nixos: 'Fondation reproductible',
      tag_plasma: 'Rust · WebAssembly',
      tag_ai: 'Runtime local typé',
      tag_telemetry: 'Zéro télémétrie',
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
      exp_h2_1: 'Une interface.',
      exp_h2_2: 'Deux surfaces.',
      exp_lead: 'Le même frontend Rust/WebAssembly s’exécute dans un navigateur ordinaire et dans un shell de bureau minimal qui possède une unique surface Wayland. L’interface est une projection de Mind, jamais un second propriétaire de l’état — c’est pourquoi elle peut être remplacée sans toucher à la mémoire, à l’identité ni à la politique.',
      feat1_title: 'Living Canvas',
      feat1_desc: 'Un frontend Rust/WebAssembly unique, compilé depuis une seule source et livré comme artefact au hachage de contenu, depuis la même origine que son API.',
      feat2_title: 'Une frontière en lecture seule',
      feat2_desc: 'La passerelle écoute uniquement en local, répond aux demandes typées de session, d’instantané et d’événements sous un budget borné, et n’expose aucune route de mutation.',
      feat3_title: 'Conçu comme un système',
      feat3_desc: 'Couleur, géométrie, mouvement, connexion et fond d’écran partagent une même grammaire visuelle, en clair comme en sombre.',
      found_h2_1: 'Déclaratif par nature.',
      found_h2_2: 'Réversible par conception.',
      found_lead: 'En dessous, Cybou hérite d’une configuration reproductible et du retour arrière par génération. Les mises à jour deviennent des états explicites — compréhensibles, testables et récupérables. Rien ne devient visible avant que son propriétaire ne l’ait validé durablement.',
      p1_head: 'Reproductible',
      p1_text: 'Reconstruire le même système depuis une configuration verrouillée.',
      p2_head: 'Récupérable',
      p2_text: 'Revenir à une génération précédente si une mise à jour échoue.',
      p3_head: 'Inspectable',
      p3_text: 'Savoir ce qui a changé avant de faire confiance à un nouvel état.',
      gen_console_title: 'États du système',
      gen147_desc: 'Actuelle · fondation de l’interface Rust',
      gen_active: 'Active',
      gen146_desc: 'Mise à jour de sécurité · vérifiée',
      gen_yesterday: 'Hier',
      gen145_desc: 'Avant la configuration graphique',
      gen_aug2: '2 août',
      prog_h2_1: 'Exécution mesurée.',
      prog_h2_2: 'Jalons vérifiés.',
      prog_lead: 'Le substrat vérifié M1–M6 porte désormais la perception ancrée, la projection épistémique, l’effacement sûr en cas de panne et la livraison gouvernée du contexte — pendant que la couche d’interface migre vers Rust et WebAssembly par étapes additives et séparément validées.',
      metric_gate_a: 'Douze services Mind isolés, avec une propriété typée explicite',
      metric_tasks: 'Trente-sept suites CTest et des contrôles de politique reproductibles',
      metric_contrast: 'Cinq crates Rust : protocole, fabric, contrats web, passerelle, Living Canvas',
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
      rm1_title: 'Fondation reproductible',
      rm2_title: 'Substrat Mind',
      rm3_title: 'Surface Rust web-first',
      rm4_title: 'Cognition ancrée et action gouvernée',
      rm1_sub: 'Builds verrouillés · système visuel Horizon · ISO',
      rm2_sub: 'M1–M6 · P6.8 · cycle de vie et reprise',
      rm3_sub: 'Crates protocole · passerelle · Living Canvas',
      rm4_sub: 'Perception · langage · agents · sécurité',
      rm_complete: 'Terminé',
      rm_inprogress: 'En cours',
      bp_banner_kicker: 'Le projet commence ici',
      bp_banner_h2_1: 'Un vrai système d’exploitation,',
      bp_banner_h2_2: 'construit une couche de confiance à la fois.',
      bp_banner_desc: 'Le blueprint définit la fondation reproductible, le runtime Mind à douze services, le chemin d’interface Rust/WebAssembly, les portes d’implémentation vérifiées et la feuille de route cognitive.',
      bp_banner_btn: 'Lire le livre blanc technique',
      btn_top: 'Retour en haut',
      footer_sub: 'Environnement pour agents · Mind local typé · interface Rust et WebAssembly.',
      bp_badge: 'Blueprint technique · architecture et feuille de route',
      bp_title: 'Cybou : un runtime cognitif gouverné avec une interface remplaçable',
      bp_lead: 'Le livre blanc actuel du runtime Mind à douze services, de son modèle de propriété et de panne, du substrat vérifié M1–M6, de la migration additive vers Rust/WebAssembly et de la feuille de route M7–M13.',
      btn_print_pdf: 'Exporter en PDF / Imprimer',
      btn_explore_landing: 'Explorer le site',
      bp_sec1_title: '1. Résumé et vision',
      toc_vision: 'Vision',
      toc_architecture: 'Architecture',
      toc_safety: 'Frontières de sûreté',
      toc_implementation: 'Implémentation',
      toc_security: 'Sécurité et licences',
      toc_faq: 'FAQ',
      bp_sec1_p1: 'Cybou est un environnement d’exploitation expérimental pour agents, composé de couches testables indépendamment : une fondation système reproductible, Mind — un runtime local typé pour la biographie durable, l’identité, les engagements, la prédiction, l’attention bornée, la santé et la reprise — et une interface Rust/WebAssembly remplaçable. Ce n’est pas un chatbot et il ne prétend à aucune conscience.',
      bp_layer1_title: 'Couche 1 : corps reproductible',
      bp_layer1_desc: 'Flakes verrouillés, générations atomiques, sorties VM/ISO/Hyper-V et portes explicites de build et de reprise. Debian 13 est l’environnement actif de build et de vérification pour toutes les portes Linux.',
      bp_layer2_title: 'Couche 2 : runtime Mind typé',
      bp_layer2_desc: 'Douze services utilisateur systemd isolés communiquent par contrats D-Bus typés. Event1 est la frontière canonique du Journal ; Presence est une projection, pas un second propriétaire. Aucun modèle de langage ni exécution privilégiée.',
      bp_layer3_title: 'Couche 3 : interface remplaçable',
      bp_layer3_desc: 'Un espace de travail Rust porte le protocole, le codec fabric et les contrats web. Une passerelle en lecture seule sur la boucle locale projette Presence en JSON typé et en flux d’événements borné, et Living Canvas l’affiche en WebAssembly dans un navigateur ou un shell de bureau.',
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
      bp_sec5_p1: 'Cybou est un logiciel libre conçu pour la souveraineté et la vie privée :'
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
      part_crypto_text: 'Поддержать Станислава Савельева напрямую в криптовалюте:',
      faq_h2_1: 'Частые',
      faq_h2_2: 'вопросы.',
      faq_q1: 'Что такое Cybou?',
      faq_a1: 'Cybou — экспериментальная операционная среда, спроектированная под агентов. Постоянный локальный рантайм Mind владеет долговременной памятью, идентичностью, обязательствами, жизненным циклом, здоровьем и свидетельствами, а модели, агенты, инструменты и интерфейсы остаются заменяемыми вокруг него.',
      faq_q2: 'Чем это отличается от чат-бота, прикрученного к Linux?',
      faq_a2: 'Модель — не Mind. Агент — не Mind. Протокол инструментов — не граница авторизации. Cybou делает память, идентичность, здоровье, жизненный цикл и восстановление явными системными сервисами с названными владельцами, поэтому никакая модель не может стать владельцем непрерывности или полномочий.',
      faq_q3: 'Нужны ли облачные сервисы и есть ли телеметрия?',
      faq_a3: 'Нет. Текущему рантайму и интерфейсу не нужны облачный аккаунт, API-ключ или хостинговый ИИ-сервис, а телеметрия не реализована вовсе. Удалённые модели могут появиться позже, но только за явной политикой контекста, чувствительности, исходящего трафика и стоимости.',
      faq_q4: 'На чём построен интерфейс?',
      faq_a4: 'Один фронтенд на Rust/WebAssembly под именем Living Canvas обслуживает и обычный браузер, и лёгкую десктопную оболочку. Он читает через шлюз на Rust, привязанный к локальной петле и работающий только на чтение: типизированные сессия, снапшот и поток событий, ни одного маршрута мутации. Прежняя оболочка на KDE/Qt — код переходного периода, а не цель.',
      faq_q5: 'Что ещё не реализовано?',
      faq_a5: 'Cybou не поставляет языковую модель, рантайм агентов или воркеров, брокер моделей и инструментов, исполнитель привилегированных действий и автономный контур безопасности. Аутентификация при старте десктопа также остаётся открытой. Это запланированные границы, и они никогда не описываются как готовые.',
      faq_q6: 'Какие открытые лицензии применяются?',
      faq_a6: 'Код и Nix-выражения Cybou — под лицензией MIT. Оригинальные визуальные ресурсы (обои, темы, SVG) — под CC BY-SA 4.0. Copyright (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Пре-релиз · идёт миграция на Rust/WebAssembly',
      hero_h1_1: 'Система, которая помнит.',
      hero_h1_2: 'Разум, который сохраняет власть.',
      hero_lead: 'Cybou — экспериментальная операционная среда, спроектированная под агентов. Двенадцать локальных сервисов дают ей долговременную память, идентичность, здоровье и восстановление, а единый интерфейс на Rust/WebAssembly показывает её в браузере или в десктопной оболочке. Модель не поставляется, облако не требуется.',
      hero_btn_explore: 'Собрать Cybou',
      hero_btn_blueprint: 'Читать технический blueprint',
      tag_nixos: 'Воспроизводимая основа',
      tag_plasma: 'Rust · WebAssembly',
      tag_ai: 'Типизированный локальный рантайм',
      tag_telemetry: 'Ноль телеметрии',
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
      exp_h2_1: 'Один интерфейс.',
      exp_h2_2: 'Две поверхности.',
      exp_lead: 'Один и тот же фронтенд на Rust/WebAssembly работает и в обычном браузере, и в минимальной десктопной оболочке, владеющей единственной поверхностью Wayland. Интерфейс — проекция Mind, а не второй владелец состояния; именно поэтому его можно заменить, не трогая память, идентичность и политику.',
      feat1_title: 'Living Canvas',
      feat1_desc: 'Единый фронтенд на Rust/WebAssembly: одна кодовая база, артефакт с хешем содержимого, отдаваемый с того же origin, что и его собственный API.',
      feat2_title: 'Граница только на чтение',
      feat2_desc: 'Шлюз слушает лишь локальную петлю, отвечает на типизированные запросы сессии, снапшота и событий в пределах заданного бюджета и не имеет ни одного маршрута мутации.',
      feat3_title: 'Спроектировано как система',
      feat3_desc: 'Цвет, геометрия, движение, экран входа и обои подчинены одной визуальной грамматике — и в тёмной, и в светлой теме.',
      found_h2_1: 'Декларативно по природе.',
      found_h2_2: 'Обратимо по замыслу.',
      found_lead: 'Под поверхностью Cybou наследует воспроизводимую конфигурацию и откат по поколениям. Обновления становятся явными состояниями системы — понятными, проверяемыми и восстановимыми. Ничто не становится видимым до того, как владелец надёжно это зафиксировал.',
      p1_head: 'Воспроизводимость',
      p1_text: 'Собрать ту же систему из зафиксированной конфигурации.',
      p2_head: 'Восстановимость',
      p2_text: 'Вернуться к прошлому поколению, если обновление не удалось.',
      p3_head: 'Прозрачность',
      p3_text: 'Знать, что изменилось, прежде чем доверять новому состоянию.',
      gen_console_title: 'Состояния системы',
      gen147_desc: 'Текущее · основа интерфейса на Rust',
      gen_active: 'Активно',
      gen146_desc: 'Обновление безопасности · проверено',
      gen_yesterday: 'Вчера',
      gen145_desc: 'До настройки графики',
      gen_aug2: '2 авг.',
      prog_h2_1: 'Измеримое исполнение.',
      prog_h2_2: 'Проверенные этапы.',
      prog_lead: 'Проверенный фундамент M1–M6 теперь несёт заземлённое восприятие, эпистемическую проекцию, устойчивое к сбоям стирание и управляемую доставку контекста, а слой интерфейса переезжает на Rust и WebAssembly аддитивными шагами с отдельными гейтами.',
      metric_gate_a: 'Двенадцать изолированных сервисов Mind с явным типизированным владением',
      metric_tasks: 'Тридцать семь наборов CTest и воспроизводимые проверки политик репозитория',
      metric_contrast: 'Пять Rust-крейтов: протокол, fabric, веб-контракты, шлюз, Living Canvas',
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
      rm1_title: 'Воспроизводимая основа',
      rm2_title: 'Фундамент Mind',
      rm3_title: 'Web-first интерфейс на Rust',
      rm4_title: 'Заземлённое познание и управляемое действие',
      rm1_sub: 'Зафиксированные сборки · визуальная система Horizon · ISO',
      rm2_sub: 'M1–M6 · P6.8 · жизненный цикл и восстановление',
      rm3_sub: 'Крейты протокола · шлюз · Living Canvas',
      rm4_sub: 'Восприятие · язык · агенты · безопасность',
      rm_complete: 'Готово',
      rm_inprogress: 'В работе',
      bp_banner_kicker: 'Проект начинается здесь',
      bp_banner_h2_1: 'Настоящая операционная система,',
      bp_banner_h2_2: 'слой доверия за слоем.',
      bp_banner_desc: 'Blueprint описывает воспроизводимую основу, рантайм Mind из двенадцати сервисов, путь интерфейса на Rust/WebAssembly, проверенные гейты реализации и когнитивную дорожную карту.',
      bp_banner_btn: 'Читать технический документ',
      btn_top: 'Наверх',
      footer_sub: 'Среда для агентов · типизированный локальный Mind · интерфейс на Rust и WebAssembly.',
      bp_badge: 'Технический blueprint · архитектура и дорожная карта',
      bp_title: 'Cybou: управляемый когнитивный рантайм с заменяемым интерфейсом',
      bp_lead: 'Актуальный технический документ о рантайме Mind из двенадцати сервисов, модели владения и отказов, проверенном фундаменте M1–M6, аддитивной миграции на Rust/WebAssembly и дорожной карте M7–M13.',
      btn_print_pdf: 'Экспорт в PDF / печать',
      btn_explore_landing: 'Открыть сайт',
      bp_sec1_title: '1. Краткое резюме и видение',
      toc_vision: 'Видение',
      toc_architecture: 'Архитектура',
      toc_safety: 'Границы безопасности',
      toc_implementation: 'Реализация',
      toc_security: 'Безопасность и лицензии',
      toc_faq: 'FAQ',
      bp_sec1_p1: 'Cybou — экспериментальная операционная среда для агентов, состоящая из независимо проверяемых слоёв: воспроизводимой системной основы, Mind — локального типизированного рантайма для долговременной биографии, идентичности, обязательств, предсказаний, ограниченного внимания, здоровья и восстановления — и заменяемого интерфейса на Rust/WebAssembly. Это не чат-бот, и он не претендует на сознание.',
      bp_layer1_title: 'Слой 1: воспроизводимое тело',
      bp_layer1_desc: 'Зафиксированные Flakes, атомарные поколения, выходы VM/ISO/Hyper-V и явные гейты сборки и восстановления. Debian 13 — активная среда сборки и проверки для всех Linux-гейтов.',
      bp_layer2_title: 'Слой 2: типизированный рантайм Mind',
      bp_layer2_desc: 'Двенадцать изолированных пользовательских сервисов systemd общаются через типизированные контракты D-Bus. Event1 — каноническая граница Журнала; Presence — проекция, а не второй владелец. Языковых моделей и привилегированного исполнения нет.',
      bp_layer3_title: 'Слой 3: заменяемый интерфейс',
      bp_layer3_desc: 'Rust-воркспейс несёт протокол, кодек fabric и веб-контракты. Шлюз только на чтение на локальной петле проецирует Presence в типизированный JSON и ограниченный поток событий, а Living Canvas отображает это как WebAssembly в браузере или десктопной оболочке.',
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
      bp_sec5_p1: 'Cybou — свободное ПО, созданное для цифрового суверенитета:'
    }
  };

  let currentLang = localStorage.getItem('cybou_lang') || 'en';

  const setLanguage = (lang) => {
    if (!translations[lang]) return;
    currentLang = lang;
    localStorage.setItem('cybou_lang', lang);

    document.querySelectorAll('[data-i18n]').forEach((node) => {
      const key = node.getAttribute('data-i18n');
      if (translations[lang][key]) {
        node.textContent = translations[lang][key];
      }
    });

    document.querySelectorAll('[data-lang-current]').forEach((node) => {
      node.textContent = lang.toUpperCase();
    });

    document.querySelectorAll('[data-lang]').forEach((btn) => {
      btn.classList.toggle('active', btn.getAttribute('data-lang') === lang);
    });

    document.documentElement.setAttribute('lang', lang);
  };

  // Check URL hash for #lang=fr or #lang=ru
  const hashMatch = window.location.hash.match(/lang=(en|fr|ru)/);
  if (hashMatch) {
    currentLang = hashMatch[1];
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
        desktopStatusText.textContent = `Workspace ${wsNum} — active`;
      }
    });
  });

  const appDockBtns = document.querySelectorAll('[data-app-dock] button');
  appDockBtns.forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const appName = btn.getAttribute('data-app');
      if (desktopStatusText) {
        desktopStatusText.textContent = `${appName} app ready`;
      }
    });
  });
})();
