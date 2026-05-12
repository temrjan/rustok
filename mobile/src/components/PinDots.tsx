/**
 * PinDots — Phase 4 M2.3.
 *
 * 6-dot indicator displaying current PIN entry progress (filled / empty)
 * с reveal animation on digit press (scale 0→1) и error animation on
 * mismatch (translateX shake + red border flash).
 *
 * Reduce-motion guard via `AccessibilityInfo.isReduceMotionEnabled()` +
 * subscription on `'reduceMotionChanged'` — animations short-circuit
 * when reduce-motion preference active. Initial async hydration race
 * accepted as best-effort trade-off per design § 2 line 150 «respect
 * reduce-motion» (mount typically completes before first user input).
 *
 * Animation scope:
 *   - Reveal scale applied ONLY к dot at index `count - 1` (just-filled).
 *     Other dots use static styling — preserves «only newest dot scales»
 *     UX semantic.
 *   - Shake (`translateX`) applied to outer row container — affects all
 *     6 dots together, correct error UX.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 line 150 (M2.3 deliverable)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.3 (PinDots a11y consumer)
 */

import React, { useEffect, useState } from 'react';
import { AccessibilityInfo, View } from 'react-native';
import Animated, {
  useAnimatedStyle,
  useSharedValue,
  withSequence,
  withTiming,
} from 'react-native-reanimated';
import clsx from 'clsx';

/** Wallet PIN length — exported для CreatePin / ConfirmPin / Unlock callers. */
export const PASSCODE_LENGTH = 6;

interface PinDotsProps {
  count: number;
  error?: boolean;
}

export function PinDots({ count, error = false }: PinDotsProps) {
  const [reduceMotion, setReduceMotion] = useState(false);

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

  const newestScale = useSharedValue(1);
  const shakeX = useSharedValue(0);

  useEffect(() => {
    if (count === 0 || reduceMotion) return;
    newestScale.value = 0;
    newestScale.value = withTiming(1, { duration: 150 });
  }, [count, reduceMotion, newestScale]);

  useEffect(() => {
    if (!error || reduceMotion) return;
    shakeX.value = withSequence(
      withTiming(-8, { duration: 50 }),
      withTiming(8, { duration: 50 }),
      withTiming(-4, { duration: 50 }),
      withTiming(4, { duration: 50 }),
      withTiming(0, { duration: 50 }),
    );
  }, [error, reduceMotion, shakeX]);

  const newestDotStyle = useAnimatedStyle(() => ({
    transform: [{ scale: newestScale.value }],
  }));

  const rowStyle = useAnimatedStyle(() => ({
    transform: [{ translateX: shakeX.value }],
  }));

  return (
    <Animated.View
      style={rowStyle}
      className="flex-row justify-center items-center"
      accessibilityLabel={`${count} of ${PASSCODE_LENGTH} digits entered`}
      accessibilityLiveRegion="polite"
    >
      {Array.from({ length: PASSCODE_LENGTH }).map((_, i) => {
        const filled = i < count;
        const isNewest = i === count - 1;
        const dotClasses = clsx(
          'w-3 h-3 rounded-full mx-2',
          filled && !error && 'bg-accent-periwinkle',
          filled && error && 'bg-semantic-danger',
          !filled && !error && 'border border-accent-periwinkle/40',
          !filled && error && 'border border-semantic-danger',
        );
        if (isNewest) {
          return (
            <Animated.View
              key={i}
              style={newestDotStyle}
              className={dotClasses}
            />
          );
        }
        return <View key={i} className={dotClasses} />;
      })}
    </Animated.View>
  );
}
