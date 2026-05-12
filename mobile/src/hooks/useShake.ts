/**
 * useShake — shared shake-on-error primitive (Phase 4 M3.3).
 *
 * Same `translateX` sequence (`-8, 8, -4, 4, 0 × 50ms = 250ms total`)
 * and `AccessibilityInfo.isReduceMotionEnabled()` guard as the inline
 * shake embedded in `<PinDots>` (M2.3). Extracted in M3.3 to share with
 * `QuizScreen` without scope-creeping PinDots.
 *
 * PinDots itself is NOT refactored to consume this hook — keeps the
 * M3.3 commit minimal. Both implementations stay visually in sync
 * because step values are identical; if either drifts, the divergence
 * is а follow-up nit to consolidate.
 *
 * Reduce-motion semantics: while `reduceMotion === true`, `triggerShake`
 * is а no-op. The async hydration race на mount (read of the OS
 * preference resolves after first paint) is the same trade-off PinDots
 * accepts per design § 2 line 150.
 *
 * @see mobile/src/components/PinDots.tsx (sibling implementation)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.6 (Animation row)
 */

import { useCallback, useEffect, useState } from 'react';
import { AccessibilityInfo } from 'react-native';
import {
  useAnimatedStyle,
  useSharedValue,
  withSequence,
  withTiming,
} from 'react-native-reanimated';

export function useShake() {
  const [reduceMotion, setReduceMotion] = useState(false);
  const translateX = useSharedValue(0);

  useEffect(() => {
    AccessibilityInfo.isReduceMotionEnabled()
      .then(setReduceMotion)
      .catch(() => {});
    const sub = AccessibilityInfo.addEventListener(
      'reduceMotionChanged',
      setReduceMotion,
    );
    return () => sub.remove();
  }, []);

  const shakeStyle = useAnimatedStyle(() => ({
    transform: [{ translateX: translateX.value }],
  }));

  const triggerShake = useCallback((): void => {
    if (reduceMotion) return;
    translateX.value = withSequence(
      withTiming(-8, { duration: 50 }),
      withTiming(8, { duration: 50 }),
      withTiming(-4, { duration: 50 }),
      withTiming(4, { duration: 50 }),
      withTiming(0, { duration: 50 }),
    );
  }, [reduceMotion, translateX]);

  return { shakeStyle, triggerShake };
}
