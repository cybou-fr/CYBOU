<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

<div align="center">

![Cybou Logo](packages/horizon-assets/cybou-aperture.svg)

# Cybou

**Умная операционная система на базе NixOS с KDE Plasma**

[![REUSE compliant](https://img.shields.io/badge/REUSE-compliant-green.svg)](https://reuse.software/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![NixOS 26.05](https://img.shields.io/badge/NixOS-26.05-blue.svg)](https://nixos.org/)

</div>

---

## О проекте

**Cybou** — это не просто desktop-окружение, а **полноценная умная операционная система** на базе NixOS с KDE Plasma 6.

Ключевая особенность — **Mind**: когнитивный движок, который предоставляет когнитивные способности через изолированные органы (identity, intention, prediction, self, workspace, presence).

---

## Состояние проекта

| Компонент | Статус |
|-----------|--------|
| **Фаза** | Phase 0 — repository bootstrap |
| **C++ Mind** | ✅ Реализован и собирается |
| **Presence Applet** | ✅ Реализован и собирается |
| **Артефакты сборки** | ✅ Очищены из истории |
| **CI** | ✅ Проверяет C++ компиляцию |
| **REUSE** | ✅ Все файлы с SPDX заголовками |

---

## Быстрый старт

```bash
# Форматирование
nix fmt

# Проверка
nix flake check

# Сборка темы
nix build .#packages.x86_64-linux.cybou-theme

# Сборка C++ пакетов
nix build .#packages.x86_64-linux.cybou-mind
nix build .#packages.x86_64-linux.cybou-presence-applet
```

---

## Архитектура

### Когнитивный движок (Mind)

- **identityd** — непрерывность субъекта между перезагрузками
- **intentiond** — обязательства, выведенные из журнала
- **predictord** — прогнозы, соединённые с результатами
- **selfd** — самооценка на основе измеренных фактов
- **workspaced** — ограниченное внимание и коалиции
- **presenced** — поверхность, показывающая содержимое журнала

### Технический стек

| Слой | Технология |
|------|------------|
| ОС | NixOS 26.05 (stable) |
| Desktop | KDE Plasma 6, Wayland, SDDM |
| Язык | C++20 / Qt6 |
| Сборка | CMake + Ninja |
| Лицензия | MIT (код), CC-BY-SA-4.0 (ассеты) |

---

## Документация

- **Спецификация**: отдельный репозиторий (authoritative)
- **ADRs**: в репозитории спецификации
- **Разработка**: см. `docs/`

---

## Лицензия

- Код: [MIT](LICENSES/MIT.txt)
- Ассеты: [CC-BY-SA-4.0](LICENSES/CC-BY-SA-4.0.txt)
- Соответствие: [REUSE 3.x](https://reuse.software/spec/)
