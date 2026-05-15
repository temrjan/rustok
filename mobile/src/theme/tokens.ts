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
  deep: '#070D1B',
} as const;

const neutral = {
  mid: '#959BB5',
  soft: '#8A8CAC',
} as const;

export const palette = {
  light: {
    canvas: '#FFFFFF',
    ink: {
      primary: '#0A1123',
      muted: '#3A3E6C',
    },
    surface: {
      alt: '#F6F7FB',
      border: '#E5E8F2',
      card: '#FFFFFF',
      elevated: '#F0F1F8',
    },
    accent,
    semantic,
    brand,
    neutral,
  },
  dark: {
    canvas: '#0A1123',
    ink: {
      primary: '#FFFFFF',
      muted: '#8A8CAC',
    },
    surface: {
      alt: '#141A33',
      border: '#242B4C',
      card: '#141A33',
      elevated: '#1C2244',
    },
    accent,
    semantic,
    brand,
    neutral,
  },
} as const;

export type ThemeMode = 'light' | 'dark';
export type Palette = (typeof palette)[ThemeMode];
