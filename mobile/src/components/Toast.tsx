/**
 * Toast — Phase 3 M2 Commit 2.
 *
 * Thin wrapper over react-native-toast-message singleton.
 *
 * Default visual config (white background, status icons) uses the
 * library defaults; theme-aware customization is deferred.
 *
 * Mount <ToastProvider /> once at the app root (after AppContent in
 * the SafeAreaProvider tree, see App.tsx).
 *
 * Usage:
 *   import { toast } from '@/components';
 *   toast.success('Saved!');
 *   toast.error('Network error', 'Try again');
 */

import Toast from 'react-native-toast-message';

type ShowFn = (text: string, title?: string) => void;

function show(type: 'success' | 'error' | 'info'): ShowFn {
  return (text: string, title?: string) => {
    // When `title` is given → text1 = title (heading), text2 = body text.
    // When `title` is omitted → text1 = body (single-line toast).
    Toast.show({
      type,
      text1: title ?? text,
      text2: title !== undefined ? text : undefined,
    });
  };
}

export const toast = {
  success: show('success'),
  error: show('error'),
  info: show('info'),
  hide: () => Toast.hide(),
};

export { default as ToastProvider } from 'react-native-toast-message';
