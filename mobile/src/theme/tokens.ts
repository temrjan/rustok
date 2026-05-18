/**
 * Design tokens — Phase 3 M1.
 *
 * Single source of truth for non-NativeWind code (StyleSheet, native APIs).
 * For className-based usage, see mobile/tailwind.config.js + mobile/global.css.
 *
 * Mirrors the values in global.css exactly. If you change one, change both.
 */

// Semantic and accent palettes are theme-agnostic for now.
// If a future design pass diverges them between modes, inline the values back
// into light/dark blocks.
const semantic = {
  success: '#22C55E',
  warn: '#F59E0B',
  danger: '#EF4444',
} as const;

const accent = {
  periwinkle: '#8387C3',
  deep: '#3A3E6C',
  soft: '#9EA3D1',
} as const;

const brand = {
  deep: '#0C0E15',
} as const;

const neutral = {
  mid: '#959BB5',
  soft: '#8A8CAC',
} as const;

export const palette = {
  light: {
    canvas: '#F2F3F7',
    ink: {
      primary: '#0A1123',
      muted: '#3A3E6C',
    },
    surface: {
      alt: '#F6F7FB',
      border: '#E5E8F2',
      card: '#FAFAFB',
      elevated: '#FCFDFE',
    },
    accent,
    semantic,
    brand,
    neutral,
  },
  dark: {
    canvas: '#11141E',
    ink: {
      primary: '#FFFFFF',
      muted: '#8A8CAC',
    },
    surface: {
      alt: '#1A1C25',
      border: '#2D2F3A',
      card: '#1A1C25',
      elevated: '#23252F',
    },
    accent,
    semantic,
    brand,
    neutral,
  },
} as const;

// Border radii — theme-invariant, NOT mirrored in global.css.
// Use via Tailwind `rounded-rw-*` (see tailwind.config.js) for className-based
// styling. For native APIs (StyleSheet borderRadius), import `radius` directly.
// `rw-` prefix avoids collision with Tailwind defaults (`rounded-md` etc.)
// which existing components rely on.
export const radius = {
  sm: 10,
  md: 14,
  lg: 18,
  xl: 24,
  pill: 9999,
} as const;

// Typography — theme-invariant, NOT mirrored in global.css.
// Weight values are strings per React Native API (`fontWeight: '400'..'900'`).
// Tailwind defaults (`font-normal/medium/semibold/bold`) already match these
// values, so no `tailwind.config.js` fontWeight extend is needed.
// `family` and `size` scale intentionally omitted — see C5 docs/DESIGN-TOKENS.md
// Known Limitations.
export const typography = {
  weight: {
    regular: '400',
    medium: '500',
    semibold: '600',
    bold: '700',
  },
} as const;

// Shadows — themed per Issue #31. `shadow.card` has {light, dark} variants;
// the dark variant uses #000000 / 0.4 opacity to keep elevation visible on
// the #1A1C25 dark card surface (the previous theme-invariant #0A1123 / 0.1
// shadow had near-zero perceptual contrast in dark mode).
// Cross-platform: iOS uses shadowColor/Offset/Opacity/Radius; Android uses
// elevation (system-rendered drop shadow). React Native silently ignores
// iOS-only fields on Android and vice-versa — one object handles both.
// Consume via `useThemedShadow('card')` hook (TS-only — no className).
export const shadow = {
  card: {
    light: {
      shadowColor: '#0A1123',
      shadowOffset: { width: 0, height: 8 },
      shadowOpacity: 0.1,
      shadowRadius: 24,
      elevation: 6,
    },
    dark: {
      shadowColor: '#000000',
      shadowOffset: { width: 0, height: 8 },
      shadowOpacity: 0.4,
      shadowRadius: 24,
      elevation: 6,
    },
  },
} as const;

export type ThemeMode = 'light' | 'dark';
export type Palette = (typeof palette)[ThemeMode];
export type RadiusKey = keyof typeof radius;
export type FontWeightKey = keyof typeof typography.weight;
export type ShadowKey = keyof typeof shadow;
