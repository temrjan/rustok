# Design Tokens — Rustok mobile

> Single source of truth for visual values across the React Native app.
> Established in Phase 3 M1 (2026-05-04, commits `4b1e641` + `2ccee00`),
> extended in **design-token foundation** (this PR, C1–C5).

---

## Contents

1. [Overview](#1-overview)
2. [Token Reference](#2-token-reference)
   - 2.1 [Brand & Accent](#21-brand--accent-theme-invariant)
   - 2.2 [Ink](#22-ink-theme-aware)
   - 2.3 [Canvas](#23-canvas-theme-aware)
   - 2.4 [Surface](#24-surface-theme-aware)
   - 2.5 [Semantic](#25-semantic-theme-invariant-tailwind-defaults)
   - 2.6 [Border Radii](#26-border-radii-theme-invariant-not-in-globalcss)
   - 2.7 [Typography](#27-typography-theme-invariant-not-in-globalcss-not-in-tailwindconfigjs)
   - 2.8 [Shadows](#28-shadows-theme-invariant-ts-only)
3. [Theme System](#3-theme-system)
4. [3-File Synchronization Invariant](#4-3-file-synchronization-invariant)
5. [Known Limitations](#5-known-limitations)
6. [Backlog / Followups](#6-backlog--followups)
7. [References](#7-references)
8. [Adding a new token](#8-adding-a-new-token)
9. [Usage Patterns](#9-usage-patterns)

---

## 1. Overview

Design tokens are **named visual constants** — colors, radii, typography weights.
Components reference these names instead of inlining literals (`#8387C3`, `12px`)
so a single change propagates everywhere without grep-and-replace.

Tokens live in **three synchronized files** in `mobile/`:

| File | Used by | What it stores |
|---|---|---|
| `src/theme/tokens.ts` | React Native StyleSheet, native APIs (direct import) | TypeScript `as const` palette + radius + typography |
| `global.css` | NativeWind v4 runtime (CSS variables) | RGB-channel values for theme-aware tokens (light/dark swap) |
| `tailwind.config.js` | NativeWind v4 className compilation | Tailwind `extend.colors` / `extend.borderRadius` referencing CSS variables |

**Invariant:** if you change a value in one file, change it in all three. See §4.

---

## 2. Token Reference

### 2.1 Brand & Accent (theme-invariant)

| Token | Value | Role | TS import | className |
|---|---|---|---|---|
| `brand.deep` | `#0C0E15` | Deepest brand layer, beyond canvas-dark | `palette.X.brand.deep` | `bg-brand-deep` |
| `accent.periwinkle` | `#8387C3` | Primary interactive, links, active icons | `palette.X.accent.periwinkle` | `bg-accent-periwinkle` |
| `accent.deep` | `#3A3E6C` | Hover, active strokes, depth | `palette.X.accent.deep` | `bg-accent-deep` |
| `accent.soft` | `#9EA3D1` | Lighter periwinkle, highlights | `palette.X.accent.soft` | `bg-accent-soft` |
| `neutral.mid` | `#959BB5` | Muted text, placeholders | `palette.X.neutral.mid` | `bg-neutral-mid` |
| `neutral.soft` | `#8A8CAC` | Dividers, inactive icons | `palette.X.neutral.soft` | `bg-neutral-soft` |

> Theme-invariant means the value is the same in both `palette.light` and `palette.dark`.
> Same hex in `:root` and `.dark:root` blocks of `global.css`.

### 2.2 Ink (theme-aware)

| Token | Light | Dark | Role |
|---|---|---|---|
| `ink.primary` | `#0A1123` | `#FFFFFF` | Primary text |
| `ink.muted` | `#3A3E6C` | `#8A8CAC` | Secondary / muted text. **Legacy compound** — see §5.1 |

className: `text-ink-primary` / `text-ink-muted`.

### 2.3 Canvas (theme-aware)

| Token | Light | Dark | Role |
|---|---|---|---|
| `canvas` | `#F2F3F7` | `#11141E` | App background, root shell |

className: `bg-canvas`.

> Light is **off-white** (not pure `#FFFFFF`) so soft-shadow cards remain
> visible against the canvas without border. Dark is **B graphite**
> (less saturated blue than the original `#0A1123`). See §5.6 for rationale.

### 2.4 Surface (theme-aware)

| Token | Light | Dark | Role |
|---|---|---|---|
| `surface.alt` | `#F6F7FB` | `#1A1C25` | Inputs, subtle fills, secondary surfaces |
| `surface.border` | `#E5E8F2` | `#2D2F3A` | Hairlines, dividers on surfaces |
| `surface.card` | `#FAFAFB` | `#1A1C25` | Cards (balance, list items) — see §5.3 |
| `surface.elevated` | `#FCFDFE` | `#23252F` | Elevated cards (modals, sheets) |

className: `bg-surface-card`, `border-surface-border`, etc.

> Light hierarchy: `canvas` (`#F2F3F7`, darkest) → `surface.card` (`#FAFAFB`)
> → `surface.elevated` (`#FCFDFE`, brightest). Visible separation without
> border via cumulative slight brightness shifts. Dark is B graphite re-tone
> applied uniformly across all surface layers. See §5.6.

### 2.5 Semantic (theme-invariant, Tailwind defaults)

| Token | Value | Role |
|---|---|---|
| `semantic.success` | `#22C55E` | Positive state, success |
| `semantic.warn` | `#F59E0B` | Caution, pending |
| `semantic.danger` | `#EF4444` | Error, destructive |

className: `text-semantic-danger`, `bg-semantic-warn/10` (Tailwind opacity syntax works).

> Current values are Tailwind defaults (`green-500`, `amber-500`, `red-500`).
> Canonical brand muted-tones (`#4AB37B`, `#D9A562`, `#E06B6B`) deferred — see §5.6.

### 2.6 Border Radii (theme-invariant, NOT in global.css)

| Token | TS value (number) | Tailwind class | Tailwind CSS value |
|---|---|---|---|
| `radius.sm` | `10` | `rounded-rw-sm` | `10px` |
| `radius.md` | `14` | `rounded-rw-md` | `14px` |
| `radius.lg` | `18` | `rounded-rw-lg` | `18px` |
| `radius.xl` | `24` | `rounded-rw-xl` | `24px` |
| `radius.pill` | `9999` | `rounded-rw-pill` | `9999px` |

> `rw-` prefix avoids collision with Tailwind defaults (`rounded-md` = 6px stays
> usable for existing components). New code uses `rounded-rw-*`. See §5.4.

TS import: `import { radius } from '../theme/tokens'; const styles = StyleSheet.create({ box: { borderRadius: radius.md } });`

### 2.7 Typography (theme-invariant, NOT in global.css, NOT in tailwind.config.js)

| Token | Value | RN fontWeight value | Tailwind class |
|---|---|---|---|
| `typography.weight.regular` | `'400'` | `'400'` / `'normal'` | `font-normal` |
| `typography.weight.medium` | `'500'` | `'500'` | `font-medium` |
| `typography.weight.semibold` | `'600'` | `'600'` | `font-semibold` |
| `typography.weight.bold` | `'700'` | `'700'` / `'bold'` | `font-bold` |

> Values are **strings** per React Native fontWeight API.
> Tailwind defaults already match — no `tailwind.config.js` `fontWeight` extend.
> `family` and `size` scale intentionally omitted — see §5.2 and §5.5.

### 2.8 Shadows (theme-invariant, TS-only)

| Token | iOS fields | Android | Role |
|---|---|---|---|
| `shadow.card` | `shadowColor: '#0A1123'`, `shadowOffset: { width: 0, height: 8 }`, `shadowOpacity: 0.1`, `shadowRadius: 24` | `elevation: 6` | Soft drop shadow for primary cards (BalanceCard hero, future modals) |

**Consumption — TS-only via inline `style`:**

```tsx
import { shadow } from '../theme/tokens';

<View
  className="bg-surface-card rounded-rw-xl p-4"
  style={shadow.card}
>
  ...
</View>
```

**Why TS-only (not Tailwind `shadow-*` extend):** NativeWind v4 maps Tailwind
`box-shadow` CSS values to RN shadow* / elevation, but the mapping loses
cross-platform precision (especially tinted shadows, exact iOS shadowOpacity).
Storing as a typed object and consuming via inline `style={...}` keeps
iOS and Android renderers consistent with values declared once.

**React Native cross-platform behaviour:** RN silently ignores iOS-only fields
(`shadowColor` / `shadowOffset` / `shadowOpacity` / `shadowRadius`) on Android,
and ignores `elevation` on iOS. One object with all fields covers both
platforms — no `Platform.select` required.

**Backlog (other levels — add when consumers arrive):**
- `shadow.soft` — subtle elevation for secondary cards / list items
- `shadow.btn` — depth for primary CTAs on light surfaces

---

## 3. Theme System

| Aspect | File / Detail |
|---|---|
| Storage | `mobile/src/stores/themeStore.ts` — Zustand store + MMKV persist (synchronous on module load) |
| Mode union | `'light' \| 'dark' \| 'system'` (re-export from `themeStore.ts`) |
| Provider | `mobile/src/components/ThemeProvider.tsx` — calls `colorScheme.set(mode)` from NativeWind v4 |
| Consumer hook | `mobile/src/hooks/useTheme.ts` — returns `{ mode, setMode }` selector |
| DOM toggle | NativeWind toggles `dark` class on root element → CSS variables in `.dark:root` swap |
| Tailwind config | `mobile/tailwind.config.js` — `darkMode: 'class'` enables class-based switch |

**Theme-aware vs theme-invariant:**

- **Theme-aware** tokens (`canvas`, `ink`, `surface`) have different values in `palette.light` and `palette.dark`. Same `--color-*` name, different RGB in `:root` vs `.dark:root`.
- **Theme-invariant** tokens (`accent`, `brand`, `neutral`, `semantic`, `radius`, `typography`) are the same in both themes. Either shared TS reference (in `palette.X`) or top-level export.

---

## 4. 3-File Synchronization Invariant

```
tokens.ts          (palette + radius + typography + types)
   ↕ mirror
global.css         (CSS variables, RGB channels, theme swap)
   ↕ reference
tailwind.config.js (Tailwind extend referencing var(--color-*))
```

**Rule:** changing one means changing all that store this token.

| Token category | tokens.ts | global.css | tailwind.config.js |
|---|---|---|---|
| Colors (theme-aware) | ✓ | ✓ | ✓ |
| Colors (theme-invariant) | ✓ | ✓ | ✓ |
| Radii | ✓ | — | ✓ |
| Typography weights | ✓ | — | — (Tailwind defaults match) |

**No automated test** currently guards the invariant. A future Jest test
(`tokens-invariant.test.ts`) parsing all three files and cross-checking is on
backlog (see §6).

---

## 5. Known Limitations

### 5.1 `ink.muted` is a legacy semantic blend

`palette.light.ink.muted = '#3A3E6C'` is identical to `accent.deep`.
`palette.dark.ink.muted = '#8A8CAC'` is identical to `neutral.soft`.

This is a Phase 3 M1 compound left for compatibility — 5 existing consumers
(`Input.tsx`, `Switch.tsx`, `AppShell.tsx` ×2) read `palette.X.ink.muted`.

**Rule for new code:** use `accent.deep` or `neutral.soft` directly. Don't extend
`ink.muted` semantics. Don't rename it — would break 5 consumers without
functional improvement.

### 5.2 `typography.family` not shipped

Custom fonts (Roboto, SF Pro Display) are **not bundled** in this codebase:
- Android `mobile/android/app/src/main/assets/fonts/` is empty
- iOS `Info.plist` has no `UIAppFonts` registration

If `fontFamily: 'Roboto, ...'` were added as a token now, React Native would
silently fall back to system default — false delivery. Family is deferred to a
separate infrastructure task (font bundling + Android assets + iOS plist +
device smoke on JFLFG6MZSSL7WCF6).

### 5.3 `surface.card` exists to close a pre-existing visual bug

`bg-surface-card` was used in `BalanceCard.tsx` (×3), `ConfirmSendScreen.tsx`,
and `ReceiveScreen.tsx` (×2) **before** any `surface.card` token existed —
silently fell back to no background. On the original dark theme this made cards
invisible against canvas (canvas-on-canvas at `#0A1123`).

C2 added `surface.card` to `palette.X.surface` + `global.css` + `tailwind.config.js`
to make `bg-surface-card` resolve. **No component changes required.**

After the `chore(theme): soften palette` re-tone, the card is visibly distinct
from canvas in both themes via colour (not only shadow):
light card `#FAFAFB` on canvas `#F2F3F7`; dark card `#1A1C25` on canvas `#11141E`.

### 5.4 Radii dual-system

Existing 13+ components use Tailwind default radii classes (`rounded-md`,
`rounded-2xl`, `rounded-full`). C3 added `rw-` prefixed classes (`rounded-rw-md`
= 14px) for canonical values from `rust-design/src/tokens.rs`.

**These two systems coexist:**
- **Existing code:** keeps Tailwind defaults (no migration in this PR).
- **New code:** uses `rounded-rw-*` for canonical brand radii.

Migration of existing components is a separate task — see §6.

### 5.5 `rw-xl` (24px) and `rw-pill` (9999px) duplicate Tailwind defaults

`Tailwind rounded-3xl` = 24px, `Tailwind rounded-full` = 9999px — same as our
`rw-xl` and `rw-pill`. Intentional: values are canonical from rust-design.

Future maintainers: if `rw-xl` ever moves to a different value (e.g. 22px),
`rounded-3xl` stays 24px — silent visual desync. Catch in `/typescript-review`
during the change.

### 5.6 Semantic colors are Tailwind defaults, not brand muted-tones

Our `semantic.success/warn/danger` (`#22C55E`/`#F59E0B`/`#EF4444`) are bright
Tailwind defaults. The rust-design canonical palette specifies brand-muted tones
(`#4AB37B`/`#D9A562`/`#E06B6B`) which are visually softer and consistent with
the periwinkle brand.

Replacement is a **visually breaking change** to existing UI and was
intentionally deferred to a separate "semantic recalibration" PR.

---

## 6. Backlog / Followups

Tracked separately after this foundation PR:

### Imminent (next PRs)

1. **PR `chore(theme): soften palette`** ✅ *(applied — see commit history on
   `chore/theme-soften`)* — two Captain-approved re-tones now live:
   - Dark canvas + dark surfaces → "B graphite" palette (6 values: canvas,
     brand.deep, surface.alt, surface.card, surface.elevated, surface.border)
   - Light canvas + light surface.card + light surface.elevated → off-white
     (3 values: `#F2F3F7`, `#FAFAFB`, `#FCFDFE`; removes pure white from
     light theme)
   Preview files: `C:\Claude\projects\Дизайн\dark-tone-preview.html` and
   `hero-block-preview.html`.

2. **PR `feat(mobile): hero block redesign`** *(planned, not yet opened)* — visual
   refactor per target design
   (`C:\Claude\projects\Дизайн\uploads\Дизайн\Снимок экрана 2026-04-27 112247.png`):
   - Shadow tokens namespace (RN iOS `shadow*` + Android `elevation` specs)
   - `BalanceCard.tsx` — single card containing balance + USD + change pill + 3
     action buttons
   - `ActionRow.tsx` — buttons inside card, `rounded-xl` (12px) squares with
     off-white surface background, dark icon
   - `WalletScreen` layout adjustments

### Later (no scheduled PR yet)

- **Custom font bundling** — register Roboto / SF Pro Display in Android
  `assets/fonts/` + iOS `UIAppFonts`. Unblocks `typography.family` token.
- **Semantic recalibration** — replace Tailwind defaults with brand muted-tones
  per rust-design canon (`#4AB37B`/`#D9A562`/`#E06B6B`). Visual breaking change,
  needs design review.
- **Migration of existing components** to canonical tokens — replace `rounded-md`
  / `rounded-2xl` with `rounded-rw-*` per component, after design pass.
- **Automated 3-file invariant Jest test** — parse `tokens.ts`, `global.css`,
  `tailwind.config.js`, cross-verify hex ↔ RGB ↔ var-references match.
- **Cross-chain / Across Protocol** — multichain support for full target
  design Networks section (Ethereum + Arbitrum + Base + Optimism + zkSync).

---

## 7. References

- **Canonical source:** [rust-design `src/tokens.rs`](https://github.com/temrjan/rust-design/blob/main/src/tokens.rs) @ commit `75cbb61` (Leptos prototype, design reference only).
- **Local clone:** `C:\Claude\projects\rust-design\` (read-only reference).
- **Target visual:** `C:\Claude\projects\Дизайн\uploads\Дизайн\Снимок экрана 2026-04-27 112247.png`.
- **Phase 3 M1 design doc:** `docs/PHASE3-DESIGN-APPSHELL.md` (foundation establishment).
- **Phase 3 M1 handoff:** `docs/PHASE3-HANDOFF.md` (commit trail `4b1e641`, `2ccee00`).
- **NativeWind v4 `.dark:root` quirk:** [nativewind/nativewind#702](https://github.com/nativewind/nativewind/issues/702).
- **React Native `fontWeight` API:** [reactnative.dev/docs/text-style-props](https://reactnative.dev/docs/text-style-props#fontweight).

---

## 8. Adding a new token

Checklist for adding a new color token:

1. **Decide theme-awareness:** does the value change between light and dark?
   - Theme-aware → add to `palette.light.X` and `palette.dark.X` in `tokens.ts`
   - Theme-invariant → add to top-level `const X = {...}` in `tokens.ts`, expose
     via `palette.light.X` and `palette.dark.X` (shared reference)
2. **Add to `global.css`** — `:root` block. If theme-aware, also `.dark:root`
   with different RGB. Use `space-separated RGB channels` format.
3. **Add to `tailwind.config.js`** — `extend.colors`, use
   `'rgb(var(--color-<name>) / <alpha-value>)'` to enable Tailwind opacity syntax.
4. **Verify naming:**
   - tokens.ts: `camelCase` (`brand.deep`)
   - global.css: `kebab-case` (`--color-brand-deep`)
   - tailwind className: `kebab-case` (`bg-brand-deep`)
5. **Run gates:** `npm run typecheck && npm run lint && npm test`. All three
   must pass with no delta vs baseline.

For radii or typography weights: skip step 2 (not in global.css), follow steps 1, 3, 4, 5.

---

## 9. Usage Patterns

Live examples for the common consumption patterns in this codebase.

### 9.1 Tailwind className (preferred — automatic theme swap)

NativeWind v4 reads CSS variables, so a single `className` works for both themes:

```tsx
import { View, Text } from 'react-native';

function BalanceCard() {
  return (
    <View className="bg-surface-card rounded-rw-md p-4">
      <Text className="text-ink-primary text-base font-semibold">
        Total balance
      </Text>
      <Text className="text-ink-muted text-xs mt-1">
        Updated 2m ago
      </Text>
    </View>
  );
}
```

- `bg-surface-card` resolves to `#FFFFFF` on light, `#141A33` on dark — no `if` needed.
- `text-ink-primary` swaps automatically `#0A1123` ↔ `#FFFFFF`.
- `rounded-rw-md` = 14px from `tokens.ts` `radius.md`.
- `font-semibold` is Tailwind default 600 — matches `typography.weight.semibold`.

### 9.2 Direct import for StyleSheet / native APIs

When NativeWind className is not available (animations, native APIs,
`StyleSheet.create` patterns), import tokens directly:

```tsx
import { StyleSheet } from 'react-native';
import { palette, radius, typography } from '../theme/tokens';
import { useTheme } from '../hooks/useTheme';

function Card() {
  const { mode } = useTheme();
  const effectiveMode = mode === 'system' ? 'light' : mode; // narrow

  const styles = StyleSheet.create({
    container: {
      backgroundColor: palette[effectiveMode].surface.card,
      borderRadius: radius.md,                     // number, e.g. 14
    },
    title: {
      color: palette[effectiveMode].ink.primary,
      fontWeight: typography.weight.semibold,      // '600'
    },
  });

  return <View style={styles.container}>...</View>;
}
```

> Note: `useTheme().mode` returns `'light' | 'dark' | 'system'`. For
> StyleSheet access into `palette`, narrow `'system'` to actual theme first
> (NativeWind v4 handles `'system'` for className but not for direct TS access).

### 9.3 Tailwind opacity syntax (alpha tints)

`global.css` stores RGB channels (`131 135 195`) instead of hex, which enables
Tailwind's opacity slash syntax for any color token:

```tsx
// 10% periwinkle background — used in active button states
<View className="bg-accent-periwinkle/10 p-3">...</View>

// 12% danger tint — used in error banners (UnlockPinScreen.tsx:257)
<View className="bg-semantic-danger/10 border border-semantic-danger p-4">
  <Text className="text-semantic-danger">Wrong PIN</Text>
</View>

// 30% accent for soft-disabled state
<View className="bg-accent-deep/30">...</View>
```

This works because `tailwind.config.js` uses `rgb(var(--color-...) / <alpha-value>)`.
No separate `*-bg` tokens needed for alpha tints.

### 9.4 Theme-invariant token in dark UI element

Some elements (like the tab bar in target design) stay dark even in light theme.
For those, use `brand.deep` directly — it's theme-invariant:

```tsx
// Tab bar — always dark, regardless of theme
<View className="bg-brand-deep border-t border-surface-border">
  <Text className="text-neutral-mid">Wallet</Text>
  <Text className="text-accent-periwinkle font-semibold">Activity</Text>
</View>
```

`brand.deep` is `#070D1B` in both themes; `neutral.mid` (`#959BB5`) and
`accent.periwinkle` (`#8387C3`) likewise theme-invariant.

### 9.5 Cross-stylesheet consistency check

If you're touching multiple files for one visual change, the synchronization rule
applies in **both directions**:

```diff
# Adding a new color "highlight":

# 1. tokens.ts
+ const highlight = { primary: '#A7AAD6' } as const;
  export const palette = {
    light: { ..., highlight },
    dark:  { ..., highlight },
  };

# 2. global.css (both blocks, even if theme-invariant)
  :root {
+   --color-highlight-primary: 167 170 214;
  }
  .dark:root {
+   --color-highlight-primary: 167 170 214;  /* same — theme-invariant */
  }

# 3. tailwind.config.js
  extend: { colors: {
+   highlight: { primary: 'rgb(var(--color-highlight-primary) / <alpha-value>)' },
  }}
```

Use the same hex throughout: TS stores hex (`'#A7AAD6'`), CSS stores RGB
channels (`167 170 214`), Tailwind stores the var-reference string.
