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
      nav_experience: 'Experience',
      nav_foundation: 'Foundation',
      nav_progress: 'Progress',
      nav_design: 'Design',
      nav_roadmap: 'Roadmap',
      nav_blueprint: 'Blueprint',
      nav_faq: 'FAQ',
      nav_partners: 'Partners',
      btn_view_blueprint: 'Blueprint',
      btn_back_home: '← Back to home',
      part_badge: 'Partnership & Support Hub',
      part_title: 'Partners, Contact & Donations',
      part_lead: 'Get in touch with creator Stanislav Saveliev, explore hardware & software partnerships, or support Cybou’s independent open-source development.',
      part_contact_head: 'Official Contact',
      part_contact_text: 'For general inquiries, press, security disclosures, distributions collaboration, or hardware support:',
      part_donate_head: 'Support & Donations',
      part_donate_text: 'Cybou is an independent open-source project. Your support funds reproducible builds, Plasma integration, Mind runtime engineering, and recovery testing.',
      part_crypto_head: 'Crypto Donations',
      part_crypto_text: 'Support Stanislav Saveliev directly via cryptocurrency:',
      faq_h2_1: 'Frequently asked',
      faq_h2_2: 'questions.',
      faq_q1: 'What is Cybou?',
      faq_a1: 'Cybou is an experimental personal operating system built on NixOS 26.05 and KDE Plasma 6 Wayland. It combines a reproducible desktop with Mind, a local typed runtime for durable memory, identity, commitments, lifecycle, and bounded attention.',
      faq_q2: 'Why is Cybou called a "Smarter Linux"?',
      faq_a2: 'Cybou makes memory, identity, health, lifecycle, and recovery explicit system services instead of hiding them in one AI model. Language models are a future optional faculty and are not part of the current runtime.',
      faq_q3: 'Does Cybou require cloud services or send telemetry?',
      faq_a3: 'The current desktop and Mind runtime require no cloud account, API key, or hosted AI service. Cybou does not yet ship an AI model; any future language faculty must remain optional and local-first.',
      faq_q4: 'How does Cybou guarantee system stability?',
      faq_a4: 'Through declarative NixOS Flakes (flake.lock) and atomic generation rollbacks. If any package update fails or breaks, you can instantly select a previous working generation at boot time.',
      faq_q5: 'What open-source licenses apply?',
      faq_a5: 'Cybou code and Nix expressions are licensed under MIT. Original visual assets (wallpapers, desktop themes, SVGs) are licensed under CC BY-SA 4.0. Copyright (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Visual foundation · v0.1',
      hero_h1_1: 'A calmer,',
      hero_h1_2: 'smarter Linux OS.',
      hero_lead: 'Cybou is a smarter personal Linux desktop built on NixOS and KDE Plasma — designed around clarity, reproducibility, zero telemetry and calm, deliberate design.',
      hero_btn_explore: 'Explore Cybou',
      hero_btn_blueprint: 'Read Technical Blueprint',
      tag_nixos: 'Built on NixOS',
      tag_plasma: 'KDE Plasma 6',
      tag_ai: 'Reproducible by design',
      tag_telemetry: 'Zero Telemetry',
      desktop_welcome: 'Welcome to Cybou',
      desktop_sub: 'A quiet system, ready when you are.',
      launcher_search: 'Search applications',
      app_files: 'Files',
      app_web: 'Web',
      app_code: 'Code',
      app_settings: 'Settings',
      gen_status: 'System is healthy',
      visual_caption: 'Interactive desktop concept — click the Cybou mark',
      exp_h2_1: 'Less noise.',
      exp_h2_2: 'More presence.',
      exp_lead: 'Cybou starts as a beautifully composed desktop, not an assistant demanding attention. Standard KDE behavior stays familiar while every visual layer feels deliberate and cohesive.',
      feat1_title: 'The Horizon desktop',
      feat1_desc: 'One floating top panel, four workspaces, no desktop clutter and a quiet atmospheric field that gives the interface room to breathe.',
      feat2_title: 'Designed as a system',
      feat2_desc: 'Color, geometry, motion, login, windows and wallpaper share one coherent visual grammar.',
      feat3_title: 'Dark and light',
      feat3_desc: 'Mineral neutrals and Aurora mint preserve clarity in both modes without chasing temporary visual trends.',
      found_h2_1: 'Declarative by nature.',
      found_h2_2: 'Reversible by design.',
      found_lead: 'Under the surface, Cybou inherits NixOS generations and reproducible configuration. Updates become explicit system states — understandable, testable and recoverable.',
      p1_head: 'Reproducible',
      p1_text: 'Build the same system from a locked configuration.',
      p2_head: 'Recoverable',
      p2_text: 'Return to a previous generation when an update fails.',
      p3_head: 'Inspectable',
      p3_text: 'Know what changed before trusting a new state.',
      gen_console_title: 'System states',
      gen147_desc: 'Current · Horizon visual foundation',
      gen_active: 'Active',
      gen146_desc: 'Security update · verified',
      gen_yesterday: 'Yesterday',
      gen145_desc: 'Before graphics configuration',
      gen_aug2: 'Aug 2',
      prog_h2_1: 'Measured execution.',
      prog_h2_2: 'Verified milestones.',
      prog_lead: 'The current repository has completed the M1–M6 runtime substrate and P6.7 latency hardening, with process, continuity, recovery, and Plasma KVM gates.',
      metric_gate_a: 'M6 recovery boundary passed in a real Plasma KVM session',
      metric_tasks: 'Nine isolated Mind daemons with typed D-Bus ownership',
      metric_contrast: 'Twenty CTest suites plus repository policy checks',
      design_sub: 'A visual system shaped by depth, negative space and a single line of light.',
      mark_desc: 'An open arc around a focused center: a system with room to evolve.',
      type_head: 'Calm, precise, human.',
      type_desc: 'System sans-serif typography keeps the site independent from remote services.',
      princ_h2_1: 'Familiar where it matters.',
      princ_h2_2: 'Original where it counts.',
      pr1_head: 'Quiet by default',
      pr1_desc: 'No animated mascot, no glowing assistant, no feed competing for attention.',
      pr2_head: 'Native KDE behavior',
      pr2_desc: 'Standard shortcuts, settings and applications stay accessible and dependable.',
      pr3_head: 'Private by construction',
      pr3_desc: 'No telemetry, no account requirement and no runtime asset downloads.',
      pr4_head: 'Built in stages',
      pr4_desc: 'A trustworthy visual foundation comes before any cognitive system is introduced.',
      road_h2_1: 'Build the body.',
      road_h2_2: 'Then grow the mind.',
      rm1_title: 'Desktop foundation',
      rm2_title: 'Mind substrate',
      rm3_title: 'Grounded perception',
      rm4_title: 'Optional faculties and action',
      bp_banner_kicker: 'The project starts here',
      bp_banner_h2_1: 'A real operating system,',
      bp_banner_h2_2: 'built one trusted layer at a time.',
      bp_banner_desc: 'The v0.1 blueprint defines the NixOS image, Plasma packages, visual system, implementation gates and acceptance criteria.',
      bp_banner_btn: 'Read Technical Whitepaper',
      btn_top: 'Back to top',
      footer_sub: 'Visual foundation · Built on NixOS and KDE Plasma.',
      bp_badge: 'Technical Blueprint · Current Architecture & Roadmap',
      bp_title: 'Cybou: A Reproducible Desktop with a Typed Cognitive Runtime',
      bp_lead: 'The current whitepaper for Cybou’s NixOS/Plasma desktop, nine-process Mind runtime, ownership model, verified M1–M6 substrate, P6.7 hardening, and M7–M9 roadmap.',
      btn_print_pdf: 'Export to PDF / Print',
      btn_explore_landing: 'Explore Web Landing',
      bp_sec1_title: '1. Executive Summary & Core Vision',
      bp_sec1_p1: 'Cybou is an experimental personal operating system with two independently testable layers: a reproducible NixOS/Plasma desktop and Mind, a local typed runtime for durable biography, identity, commitments, prediction, bounded attention, health, and recovery. It is not a chatbot and makes no claim of consciousness.',
      bp_layer1_title: 'Layer 1: Reproducible Body',
      bp_layer1_desc: 'NixOS 26.05, locked Flakes, KDE Plasma 6 Wayland, Horizon packages, VM/ISO/Hyper-V outputs, atomic generations, and explicit build and recovery gates.',
      bp_layer2_title: 'Layer 2: Typed Mind Runtime',
      bp_layer2_desc: 'Nine isolated systemd user services communicate through typed Qt/D-Bus contracts. Event1 is the canonical Journal boundary; Presence is a projection, not a second owner. Language and privileged execution remain absent.',
      bp_sec2_title: '2. Architecture & Technical Stack',
      bp_sec2_p1: 'Cybou separates Body, Mind, and Presence. Durable state belongs to explicit owners; the shell is a remote cache and presentation boundary. Cross-owner commands are bounded and fail closed.',
      bp_stack_title: 'Core System Stack',
      bp_sec3_title: '3. Cognitive Contracts and Safety Boundaries',
      bp_sec3_p1: 'The implemented substrate keeps cognition inspectable without turning a model, UI, or coordinator into an unbounded owner. Future capabilities must preserve these contracts:',
      bp_ai_point1: '<strong>Ownership before intelligence:</strong> model ≠ identity, UI ≠ Mind, attention ≠ biography, and proposal ≠ authorization.',
      bp_ai_point2: '<strong>Durability before visibility:</strong> state is projected only after its owner commits it; consolidation adds evidence-linked outcomes and never rewrites history.',
      bp_ai_point3: '<strong>Bounded degradation:</strong> Health1 publishes typed deficits and recovery progress; compound Presence reads and mutations share one monotonic deadline.',
      bp_ai_point4: '<strong>No hidden agency:</strong> M8 language is optional and replaceable; M9 planning, authorization, execution, and observed outcomes remain separate boundaries.',
      bp_sec4_title: '4. Implementation Status & Progress Matrix',
      bp_sec4_p1: 'Development progress is tracked rigorously across milestone phases and verified against automated acceptance gates.',
      bp_sec5_title: '5. Security, Privacy & Licensing',
      bp_sec5_p1: 'Cybou is free software built for sovereignty and privacy:'
    },
    fr: {
      nav_experience: 'Expérience',
      nav_foundation: 'Fondation',
      nav_progress: 'Progrès',
      nav_design: 'Design',
      nav_roadmap: 'Feuille de route',
      nav_blueprint: 'Blueprint',
      nav_faq: 'FAQ',
      nav_partners: 'Partenaires',
      btn_view_blueprint: 'Blueprint',
      btn_back_home: '← Retour à l’accueil',
      part_badge: 'Espace Partenariat & Soutien',
      part_title: 'Partenaires, Contact & Dons',
      part_lead: 'Contactez le créateur Stanislav Saveliev, explorez les partenariats matériels et logiciels, ou soutenez le développement indépendant de Cybou.',
      part_contact_head: 'Contact Officiel',
      part_contact_text: 'Pour toute demande générale, presse, sécurité, collaboration ou support matériel :',
      part_donate_head: 'Soutien & Dons',
      part_donate_text: 'Cybou est un projet open source indépendant. Votre soutien finance les builds reproductibles, l’intégration Plasma, le runtime Mind et les tests de reprise.',
      part_crypto_head: 'Dons en Cryptomonnaie',
      part_crypto_text: 'Soutenez directement Stanislav Saveliev via cryptomonnaie :',
      faq_h2_1: 'Foire aux',
      faq_h2_2: 'questions.',
      faq_q1: 'Qu’est-ce que Cybou ?',
      faq_a1: 'Cybou est un système d’exploitation personnel expérimental basé sur NixOS 26.05 et KDE Plasma 6 Wayland. Il associe un bureau reproductible à Mind, un runtime local typé pour la mémoire durable, l’identité, les engagements et le cycle de vie.',
      faq_q2: 'Pourquoi Cybou est-il qualifié de « Linux plus intelligent » ?',
      faq_a2: 'Cybou rend explicites la mémoire, l’identité, la santé, le cycle de vie et la reprise au lieu de les cacher dans un modèle IA unique. Les modèles de langage sont une faculté future optionnelle.',
      faq_q3: 'Cybou nécessite-t-il des services cloud ou envoie-t-il de la télémétrie ?',
      faq_a3: 'Le bureau et le runtime Mind actuels ne nécessitent ni compte cloud, ni clé API, ni service IA hébergé. Cybou ne livre pas encore de modèle IA ; toute future faculté linguistique devra rester optionnelle et locale.',
      faq_q4: 'Comment Cybou garantit-il la stabilité du système ?',
      faq_a4: 'Grâce à la gestion déclarative NixOS Flakes (flake.lock) et aux retours en arrière atomiques. En cas de problème lors d’une mise à jour, vous pouvez revenir instantanément à une génération antérieure au démarrage.',
      faq_q5: 'Quelles sont les licences open source applicables ?',
      faq_a5: 'Le code source et les expressions Nix sont sous licence MIT. Les ressources visuelles d’origine sont sous CC BY-SA 4.0. Droits d’auteur (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Fondation visuelle · v0.1',
      hero_h1_1: 'Un système Linux',
      hero_h1_2: 'plus calme et intelligent.',
      hero_lead: 'Cybou est un bureau Linux personnel plus intelligent, bâti sur NixOS et KDE Plasma — pensé pour la clarté, la reproductibilité, zéro télémétrie et un design calme et délibéré.',
      hero_btn_explore: 'Explorer Cybou',
      hero_btn_blueprint: 'Lire le blueprint technique',
      tag_nixos: 'Basé sur NixOS',
      tag_plasma: 'KDE Plasma 6',
      tag_ai: 'Reproductible par conception',
      tag_telemetry: 'Zéro télémétrie',
      desktop_welcome: 'Bienvenue sur Cybou',
      desktop_sub: 'Un système apaisé, prêt quand vous l’êtes.',
      launcher_search: 'Rechercher des applications',
      app_files: 'Fichiers',
      app_web: 'Web',
      app_code: 'Code',
      app_settings: 'Paramètres',
      gen_status: 'Système en parfait état',
      visual_caption: 'Concept de bureau interactif — cliquez sur le symbole Cybou',
      exp_h2_1: 'Moins de bruit.',
      exp_h2_2: 'Plus de présence.',
      exp_lead: 'Cybou commence par un bureau magnifiquement composé, pas un assistant exigeant de l’attention. L’ergonomie KDE reste familière tout en offrant une identité visuelle sobre et cohérente.',
      feat1_title: 'Le bureau Horizon',
      feat1_desc: 'Un panneau supérieur flottant, quatre espaces de travail, aucun encombrement et un champ atmosphérique apaisant.',
      feat2_title: 'Conçu comme un système',
      feat2_desc: 'Couleurs, géométrie, animations, écran de connexion et fond d’écran partagent la même grammaire visuelle.',
      feat3_title: 'Sombre et clair',
      feat3_desc: 'Des tons minéraux et la menthe Aurora préservent la lisibilité dans les deux modes sans suivre de tendances éphémères.',
      found_h2_1: 'Déclaratif par nature.',
      found_h2_2: 'Réversible par conception.',
      found_lead: 'Sous la surface, Cybou hérite des générations NixOS et d’une configuration reproductible. Chaque mise à jour devient un état système explicite et vérifiable.',
      p1_head: 'Reproductible',
      p1_text: 'Reconstruisez exactement le même système à partir d’une configuration verrouillée.',
      p2_head: 'Rétablissable',
      p2_text: 'Revenez instantanément à une génération précédente en cas de problème.',
      p3_head: 'Inspectable',
      p3_text: 'Sachez ce qui a changé avant de valider un nouvel état.',
      gen_console_title: 'États du système',
      gen147_desc: 'Actuel · Fondation visuelle Horizon',
      gen_active: 'Actif',
      gen146_desc: 'Mise à jour de sécurité · vérifiée',
      gen_yesterday: 'Hier',
      gen145_desc: 'Avant configuration graphique',
      gen_aug2: '2 Août',
      prog_h2_1: 'Exécution mesurée.',
      prog_h2_2: 'Jalons vérifiés.',
      prog_lead: 'Le dépôt actuel a terminé le socle M1–M6 et le durcissement P6.7, avec des gates processus, continuité, reprise et Plasma KVM.',
      metric_gate_a: 'Gate de reprise M6 validée dans une session Plasma KVM réelle',
      metric_tasks: 'Neuf démons Mind isolés avec ownership D-Bus typé',
      metric_contrast: 'Vingt suites CTest et contrôles de politique du dépôt',
      design_sub: 'Un système visuel façonné par la profondeur, l’espace négatif et une ligne de lumière.',
      mark_desc: 'Un arc ouvert autour d’un centre focalisé : un système prêt à évoluer.',
      type_head: 'Calme, précis, humain.',
      type_desc: 'La typographie système sans-serif garantit l’indépendance vis-à-vis des services distants.',
      princ_h2_1: 'Familiers là où ça compte.',
      princ_h2_2: 'Original là où ça fait la différence.',
      pr1_head: 'Silencieux par défaut',
      pr1_desc: 'Pas de mascotte animée, pas d’assistant intrusif, pas de flux captant l’attention.',
      pr2_head: 'Comportement KDE natif',
      pr2_desc: 'Raccourcis, paramètres et applications standard restent accessibles et fiables.',
      pr3_head: 'Privé par construction',
      pr3_desc: 'Aucune télémétrie, aucune exigence de compte ni de téléchargement distant au runtime.',
      pr4_head: 'Construit par étapes',
      pr4_desc: 'Une fondation visuelle et technique de confiance avant d’introduire tout système cognitif.',
      road_h2_1: 'Construire le corps.',
      road_h2_2: 'Puis éveiller l’esprit.',
      rm1_title: 'Fondation du bureau',
      rm2_title: 'Socle Mind',
      rm3_title: 'Perception fondée',
      rm4_title: 'Facultés et action optionnelles',
      bp_banner_kicker: 'Le projet commence ici',
      bp_banner_h2_1: 'Un véritable système d’exploitation,',
      bp_banner_h2_2: 'construit couche par couche.',
      bp_banner_desc: 'Le plan v0.1 définit l\'image NixOS, les paquets Plasma, le système visuel, les jalons d\'implémentation et les critères d\'acceptation.',
      bp_banner_btn: 'Lire le livre blanc technique',
      btn_top: 'Haut de page',
      footer_sub: 'Fondation visuelle · Bâti sur NixOS et KDE Plasma.',
      bp_badge: 'Blueprint technique · Architecture actuelle & Feuille de route',
      bp_title: 'Cybou : un bureau reproductible avec un runtime cognitif typé',
      bp_lead: 'Le livre blanc actuel du bureau NixOS/Plasma, du runtime Mind à neuf processus, de son modèle de propriété, du socle M1–M6 vérifié, du durcissement P6.7 et de la feuille de route M7–M9.',
      btn_print_pdf: 'Exporter en PDF / Imprimer',
      btn_explore_landing: 'Explorer la présentation',
      bp_sec1_title: '1. Résumé exécutif & Vision',
      bp_sec1_p1: 'Cybou est un système personnel expérimental composé de deux couches testables séparément : un bureau NixOS/Plasma reproductible et Mind, un runtime local typé pour la biographie durable, l’identité, les engagements, l’attention, la santé et la reprise. Ce n’est pas un chatbot.',
      bp_layer1_title: 'Couche 1 : Corps reproductible',
      bp_layer1_desc: 'NixOS 26.05, Flakes verrouillés, KDE Plasma 6 Wayland, paquets Horizon, sorties VM/ISO/Hyper-V, générations atomiques et gates de build et de reprise.',
      bp_layer2_title: 'Couche 2 : Runtime Mind typé',
      bp_layer2_desc: 'Neuf services utilisateur systemd isolés communiquent par contrats Qt/D-Bus typés. Event1 est la frontière canonique du Journal ; Presence reste une projection. Le langage et l’exécution privilégiée sont absents.',
      bp_sec2_title: '2. Architecture & Stack technique',
      bp_sec2_p1: 'Cybou sépare Body, Mind et Presence. L’état durable appartient à des propriétaires explicites ; le shell est un cache distant et une frontière de présentation. Les commandes multi-propriétaires sont bornées et échouent fermées.',
      bp_stack_title: 'Stack système principal',
      bp_sec3_title: '3. Contrats cognitifs et limites de sécurité',
      bp_sec3_p1: 'Le socle garde la cognition inspectable sans transformer un modèle, une UI ou un coordinateur en propriétaire illimité. Les capacités futures doivent préserver ces contrats :',
      bp_ai_point1: '<strong>La propriété avant l’intelligence :</strong> modèle ≠ identité, UI ≠ Mind, attention ≠ biographie, proposition ≠ autorisation.',
      bp_ai_point2: '<strong>La durabilité avant la visibilité :</strong> un état n’est projeté qu’après validation par son propriétaire ; la consolidation ne réécrit jamais l’histoire.',
      bp_ai_point3: '<strong>Dégradation bornée :</strong> Health1 publie les déficits et la reprise ; les lectures et mutations composées de Presence partagent un délai monotone.',
      bp_ai_point4: '<strong>Aucune agence cachée :</strong> le langage M8 reste optionnel ; planification, autorisation, exécution et résultat observé M9 restent séparés.',
      bp_sec4_title: '4. Matrice de progrès & État d’avancement',
      bp_sec4_p1: 'Le développement est suivi rigoureusement à travers des jalons vérifiés par des tests automatisés.',
      bp_sec5_title: '5. Sécurité, Vie privée & Licences',
      bp_sec5_p1: 'Cybou est un logiciel libre conçu pour la souveraineté et la vie privée :'
    },
    ru: {
      nav_experience: 'Интерфейс',
      nav_foundation: 'Архитектура',
      nav_progress: 'Прогресс',
      nav_design: 'Дизайн',
      nav_roadmap: 'Планы',
      nav_blueprint: 'Blueprint',
      nav_faq: 'FAQ',
      nav_partners: 'Партнёрам',
      btn_view_blueprint: 'Blueprint',
      btn_back_home: '← На главную',
      part_badge: 'Центр сотрудничества и поддержки',
      part_title: 'Партнёры, контакты и донаты',
      part_lead: 'Свяжитесь с автором проекта Станиславом Савельевым, обсудите аппаратные и программные партнёрства или поддержите независимую разработку Cybou.',
      part_contact_head: 'Официальный контакт',
      part_contact_text: 'По общим вопросам, прессе, безопасности, сотрудничеству дистрибутивов или поддержке железа:',
      part_donate_head: 'Поддержка и донаты',
      part_donate_text: 'Cybou — независимый open-source проект. Поддержка финансирует воспроизводимые сборки, интеграцию Plasma, runtime Mind и тесты восстановления.',
      part_crypto_head: 'Крипто-донаты',
      part_crypto_text: 'Поддержите Станислава Савельева напрямую через криптовалюту:',
      faq_h2_1: 'Часто задаваемые',
      faq_h2_2: 'вопросы.',
      faq_q1: 'Что такое Cybou?',
      faq_a1: 'Cybou — экспериментальная персональная операционная система на NixOS 26.05 и KDE Plasma 6 Wayland. Она объединяет воспроизводимый рабочий стол и Mind — локальный типизированный runtime памяти, идентичности, обязательств и жизненного цикла.',
      faq_q2: 'Почему Cybou называют «более умным Linux»?',
      faq_a2: 'Cybou делает память, идентичность, здоровье, жизненный цикл и восстановление явными системными сервисами, а не скрывает их в одной модели ИИ. Языковые модели — будущая необязательная функция.',
      faq_q3: 'Требуются ли Cybou облачные сервисы или отправка телеметрии?',
      faq_a3: 'Текущим рабочему столу и runtime Mind не нужны облачный аккаунт, API-ключ или внешний ИИ-сервис. Cybou пока не поставляет модель ИИ; будущая языковая функция должна оставаться необязательной и local-first.',
      faq_q4: 'Как Cybou гарантирует стабильность системы?',
      faq_a4: 'Благодаря декларативной конфигурации NixOS Flakes (flake.lock) и атомарным откатам поколений. Если обновление содержит ошибку, при загрузке можно мгновенно выбрать заведомо рабочее состояние.',
      faq_q5: 'Под какими лицензиями распространяется проект?',
      faq_a5: 'Исходный код и Nix-файлы лицензированы под MIT. Оригинальные визуальные артефакты — под CC BY-SA 4.0. Авторские права (c) 2026 Stanislav Saveliev.',
      hero_eyebrow: 'Визуальный фундамент · v0.1',
      hero_h1_1: 'Спокойный и умный',
      hero_h1_2: 'дистрибутив Linux.',
      hero_lead: 'Cybou — это более умный персональный рабочий стол Linux на NixOS и KDE Plasma, построенный вокруг ясности, воспроизводимости, отсутствия телеметрии и спокойного продуманного дизайна.',
      hero_btn_explore: 'Исследовать Cybou',
      hero_btn_blueprint: 'Читать спецификацию',
      tag_nixos: 'На базе NixOS',
      tag_plasma: 'KDE Plasma 6',
      tag_ai: 'Воспроизводимость по умолчанию',
      tag_telemetry: 'Нулевая телеметрия',
      desktop_welcome: 'Добро пожаловать в Cybou',
      desktop_sub: 'Спокойная система, готовая к работе.',
      launcher_search: 'Поиск приложений',
      app_files: 'Файлы',
      app_web: 'Браузер',
      app_code: 'Код',
      app_settings: 'Настройки',
      gen_status: 'Система стабильна',
      visual_caption: 'Интерактивный концепт рабочего стола — нажмите на логотип Cybou',
      exp_h2_1: 'Меньше шума.',
      exp_h2_2: 'Больше фокуса.',
      exp_lead: 'Cybou начинается как изящный рабочий стол, а не навязчивый ассистент. Привычное поведение KDE сочетается с продуманным и цельным визуальным языком.',
      feat1_title: 'Рабочий стол Horizon',
      feat1_desc: 'Одна плавающая верхняя панель, четыре рабочих стола, отсутствие лишних иконок и атмосфера спокойствия.',
      feat2_title: 'Единая система',
      feat2_desc: 'Цвет, геометрия, анимации, экран входа, окна и обои следуют единому визуальному коду.',
      feat3_title: 'Темная и светлая темы',
      feat3_desc: 'Минеральные оттенки и мятный акцент Aurora обеспечивают читаемость без погони за временными трендами.',
      found_h2_1: 'Декларативный.',
      found_h2_2: 'Обратимый по дизайну.',
      found_lead: 'В основе Cybou — поколения NixOS и воспроизводимая конфигурация. Обновления становятся ясными и проверяемыми состояниями системы.',
      p1_head: 'Воспроизводимость',
      p1_text: 'Сборка той же системы из зафиксированной конфигурации.',
      p2_head: 'Восстанавливаемость',
      p2_text: 'Мгновенный откат к предыдущему поколению при сбоях.',
      p3_head: 'Прозрачность',
      p3_text: 'Точное понимание изменений до применения нового состояния.',
      gen_console_title: 'Состояния системы',
      gen147_desc: 'Текущее · Визуальный фундамент Horizon',
      gen_active: 'Активно',
      gen146_desc: 'Обновление безопасности · проверено',
      gen_yesterday: 'Вчера',
      gen145_desc: 'До настройки графики',
      gen_aug2: '2 авг',
      prog_h2_1: 'Точное исполнение.',
      prog_h2_2: 'Проверенные этапы.',
      prog_lead: 'В репозитории завершены фундамент M1–M6 и hardening P6.7, включая process-, continuity-, recovery- и Plasma KVM-gates.',
      metric_gate_a: 'M6 recovery gate пройден в реальной Plasma KVM-сессии',
      metric_tasks: 'Девять изолированных Mind-демонов с типизированным D-Bus ownership',
      metric_contrast: 'Двадцать наборов CTest и policy-проверки репозитория',
      design_sub: 'Визуальная система, сформированная глубиной, пространством и тонкой линией света.',
      mark_desc: 'Открытая дуга вокруг фокусного центра — система, готовая развиваться.',
      type_head: 'Спокойная, точная, человечная.',
      type_desc: 'Системная типографика без зависимости от сторонних шрифтовых сервисов.',
      princ_h2_1: 'Привычный там, где нужно.',
      princ_h2_2: 'Оригинальный там, где важно.',
      pr1_head: 'Тихий по умолчанию',
      pr1_desc: 'Никаких анимационных маскотов, горящих значков и всплывающих уведомлений.',
      pr2_head: 'Нативное поведение KDE',
      pr2_desc: 'Стандартные сочетания клавиш, настройки и приложения остаются привычными.',
      pr3_head: 'Конфиденциальность в основе',
      pr3_desc: 'Полное отсутствие телеметрии, требований учетных записей и сетевых загрузок.',
      pr4_head: 'Поэтапное развитие',
      pr4_desc: 'Надежное "тело" системы строится до внедрения активных интеллектуальных сервисов.',
      road_h2_1: 'Создать тело.',
      road_h2_2: 'Затем выстроить разум.',
      rm1_title: 'Фундамент рабочего стола',
      rm2_title: 'Фундамент Mind',
      rm3_title: 'Обоснованное восприятие',
      rm4_title: 'Необязательные функции и действия',
      bp_banner_kicker: 'Проект начинается здесь',
      bp_banner_h2_1: 'Настоящая операционная система,',
      bp_banner_h2_2: 'создаваемая слой за слоем.',
      bp_banner_desc: 'План v0.1 определяет образ NixOS, пакеты Plasma, визуальную систему, этапы реализации и критерии приёмки.',
      bp_banner_btn: 'Читать спецификацию Whitepaper',
      btn_top: 'Наверх',
      footer_sub: 'Визуальная основа · На базе NixOS и KDE Plasma.',
      bp_badge: 'Технический Blueprint · Текущая архитектура и план',
      bp_title: 'Cybou: воспроизводимый рабочий стол и типизированный когнитивный runtime',
      bp_lead: 'Актуальный whitepaper рабочего стола NixOS/Plasma, девятипроцессного runtime Mind, модели ownership, проверенного фундамента M1–M6, hardening P6.7 и плана M7–M9.',
      btn_print_pdf: 'Экспорт в PDF / Печать',
      btn_explore_landing: 'Открыть главную страницу',
      bp_sec1_title: '1. Концепция и ключевое видение',
      bp_sec1_p1: 'Cybou — экспериментальная персональная система из двух независимо тестируемых слоёв: воспроизводимого рабочего стола NixOS/Plasma и Mind, локального типизированного runtime долговечной биографии, идентичности, обязательств, внимания, здоровья и восстановления. Это не чат-бот.',
      bp_layer1_title: 'Слой 1: Воспроизводимое тело',
      bp_layer1_desc: 'NixOS 26.05, зафиксированные Flakes, KDE Plasma 6 Wayland, пакеты Horizon, VM/ISO/Hyper-V, атомарные поколения и явные gates сборки и восстановления.',
      bp_layer2_title: 'Слой 2: Типизированный runtime Mind',
      bp_layer2_desc: 'Девять изолированных пользовательских systemd-сервисов общаются через типизированные Qt/D-Bus контракты. Event1 — каноническая граница Journal, а Presence — только проекция. Язык и привилегированное исполнение отсутствуют.',
      bp_sec2_title: '2. Архитектура и стек технологий',
      bp_sec2_p1: 'Cybou разделяет Body, Mind и Presence. Долговечным состоянием владеют явные owners; shell остаётся удалённым кэшем и presentation boundary. Составные команды ограничены и fail closed.',
      bp_stack_title: 'Основной стек системы',
      bp_sec3_title: '3. Когнитивные контракты и границы безопасности',
      bp_sec3_p1: 'Реализованный фундамент сохраняет проверяемость cognition и не превращает модель, UI или coordinator в неограниченного owner. Будущие функции обязаны сохранять контракты:',
      bp_ai_point1: '<strong>Ownership раньше intelligence:</strong> model ≠ identity, UI ≠ Mind, attention ≠ biography, proposal ≠ authorization.',
      bp_ai_point2: '<strong>Durability раньше visibility:</strong> состояние проецируется только после commit его owner; consolidation не переписывает историю.',
      bp_ai_point3: '<strong>Ограниченная деградация:</strong> Health1 публикует deficits и recovery; составные чтения и mutations Presence используют один монотонный deadline.',
      bp_ai_point4: '<strong>Никакой скрытой agency:</strong> язык M8 необязателен; planning, authorization, execution и observed outcome M9 остаются раздельными.',
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
    button.addEventListener('click', () => {
      const item = button.closest('.faq-item');
      const open = !item?.classList.contains('open');
      document.querySelectorAll('.faq-item').forEach((el) => el.classList.remove('open'));
      item?.classList.toggle('open', open);
    });
  });

  const updateHeader = () => header?.classList.toggle('is-scrolled', window.scrollY > 24);
  updateHeader();
  window.addEventListener('scroll', updateHeader, { passive: true });

  menuButton?.addEventListener('click', () => {
    const open = !mobileNav.classList.contains('open');
    mobileNav.classList.toggle('open', open);
    menuButton.setAttribute('aria-expanded', String(open));
  });

  mobileNav?.querySelectorAll('a').forEach((link) => {
    link.addEventListener('click', () => {
      mobileNav.classList.remove('open');
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
    if (event.key === 'Escape') setLauncher(false);
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

