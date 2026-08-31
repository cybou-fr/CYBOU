# ADR-0007: REUSE 3.x Compliance

## Status
Accepted

## Context
Cybou is a project with both code (MIT license) and assets (CC-BY-SA-4.0 license). To ensure proper license compliance and make it easy for users to understand the licensing of each file, we need a standardized approach.

REUSE (Software for Reuse) provides a standard specification for documenting license and copyright information in software projects. Version 3.x is the current standard.

## Decision
**Adopt REUSE 3.x specification for all source files.**

This means:
1. Every source file must contain an SPDX license identifier
2. Every source file must contain copyright information
3. License texts must be included in LICENSES/ directory
4. All files must be covered by a known license

## Implementation

### SPDX License Identifiers
All source files must start with:
```c
// SPDX-FileCopyrightText: <year> <copyright holder>
// SPDX-License-Identifier: MIT
```

Or for non-code files:
```markdown
<!--
SPDX-FileCopyrightText: <year> <copyright holder>
SPDX-License-Identifier: MIT
-->
```

### License Files
- `LICENSES/MIT.txt` - MIT License text
- `LICENSES/CC-BY-SA-4.0.txt` - Creative Commons Attribution-ShareAlike 4.0 text

### License Assignment
- **Code files** (.rs, .toml, .js, .html, .css): MIT License
- **Asset files** (images, themes, etc.): CC-BY-SA-4.0 License
- **Documentation**: MIT License (same as code)

### Validation
REUSE compliance is validated as part of CI:
```toml
# In Cargo.toml
checks.x86_64-linux.reuse = ...
```

This runs `reuse lint` to verify all files have proper SPDX headers.

## Consequences

### Positive
- Clear license information for every file
- Easy for users to understand what they can do with each file
- Automated compliance checking
- Industry standard approach
- Required for some distributions and organizations

### Negative
- Requires discipline to add headers to all new files
- Slightly more verbose files
- Need to maintain LICENSES/ directory

## Enforcement

### CI Check
The CI workflow includes REUSE validation. Any file without proper SPDX headers will cause the check to fail.

### Gate A
REUSE compliance is a repository gate documented in `../TESTING.md`. No file can be merged without
proper license metadata.

### Developer Guidelines
- Always add SPDX headers to new files
- Use correct license for file type (MIT for code, CC-BY-SA-4.0 for assets)
- Run `reuse lint` locally before committing

## Related
- checks.yml - CI includes REUSE validation
- LICENSES/ - License text files
- ADR-0003 - Fail-closed principle (nothing shown without measurement, including licenses)
