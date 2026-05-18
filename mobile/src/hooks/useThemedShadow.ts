/**
 * useThemedShadow — Issue #31 themed shadow resolution.
 *
 * Returns the {light, dark} variant of a `shadow` token for native-API
 * style consumers (`style={...}` — RN ignores Tailwind shadow utilities,
 * so there is no className path). Mirrors ActionRow.tsx 'system' →
 * 'light' | 'dark' resolution: honors an explicit `themeStore.mode`,
 * else defers to OS via `useColorScheme()` (reactive — re-renders on
 * OS theme switch).
 */

import { useColorScheme, type ViewStyle } from 'react-native';
import { useThemeStore } from '../stores/themeStore';
import { shadow, type ShadowKey } from '../theme/tokens';

export function useThemedShadow(key: ShadowKey): ViewStyle {
  const storeMode = useThemeStore((s) => s.mode);
  const osScheme = useColorScheme();
  const mode: 'light' | 'dark' =
    storeMode === 'system' ? (osScheme === 'dark' ? 'dark' : 'light') : storeMode;
  return shadow[key][mode];
}
