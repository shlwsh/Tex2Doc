---
name: commercial-ui-design
description: Use when Codex needs to audit, design, or refactor Flutter or Slint frontends into commercial-grade product interfaces with reusable design systems, theme tokens, i18n/l10n, responsive layouts, polished states, and validation.
---

# Commercial UI Design

## Workflow

1. Audit existing screens, component structure, hardcoded styles, hardcoded text, responsive behavior, empty/loading/error states, and theme/i18n support.
2. Preserve business logic and callback/API contracts. Move presentation concerns into reusable components and tokens.
3. Define design tokens before page work: colors, typography, spacing, radii, borders, elevation, motion, and state colors.
4. Build or extend reusable primitives before refactoring pages: scaffold, top bar, sidebar/navigation, card, button, icon button, text field, select, dialog, snackbar/toast, table/list, empty/loading/error states.
5. Extract user-visible copy into locale resources or a centralized translation module. Support at least `zh-CN` and `en-US`; use stable keys such as `common.confirm`, `nav.dashboard`, `settings.theme`, `error.network`.
6. Refactor primary pages into product workflows, not demo blocks. Keep dense operational SaaS layouts calm, scannable, and utilitarian.
7. Verify light mode, dark mode, theme switching, long English text, Chinese text, mobile/tablet/desktop widths, disabled states, permission-denied states, loading states, empty states, and errors.
8. Run formatting, static analysis, UI tests, and relevant framework build/check commands.
9. Write a short design report under the project docs when requested, including before/after issues, token decisions, component inventory, i18n coverage, and remaining gaps.

## Flutter Rules

- Use `ThemeData`, `ColorScheme`, `TextTheme`, and `ThemeExtension` for the design system.
- Prefer files named like `app_theme.dart`, `app_tokens.dart`, `app_i18n.dart`, and `app_components.dart`.
- Use Material 3 consistently. Avoid raw `Colors.*` in page code except inside token/theme definitions.
- Use `LayoutBuilder`, `Wrap`, `GridView`, `Flex`, and constrained content widths for responsive layout.
- Persist language and theme selections when the project already has or can accept a lightweight settings store.
- Keep visible strings out of widgets unless they are data values from APIs or user files.
- Use concise motion between 120ms and 240ms.

## Slint Rules

- Keep visual constants in global tokens or top-level window properties that Rust can theme.
- Prefer reusable Slint components for repeated card, metric, status, input, and action-row patterns.
- Expose text as properties or bind it from Rust i18n; do not bury product copy in repeated page components.
- Use `states`, `enabled`, visual feedback, and short transitions for hover/pressed/disabled/selected where supported.
- Keep callback signatures stable unless the task explicitly includes backend changes.

## Visual Standard

- Use quiet surfaces, clear borders, modest radius, restrained shadows, and strong alignment.
- Prefer toolbar/sidebar/dashboard patterns for SaaS and desktop tools.
- Avoid landing-page hero layouts, decorative gradients, oversized marketing cards, and one-note palettes.
- Every primary workflow should show normal, loading, empty, error, disabled, and permission-denied affordances.
