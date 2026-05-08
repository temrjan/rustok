/**
 * PinPad — Phase 4 M2.3.
 *
 * 3×4 numeric keypad для PIN entry. Layout:
 *   1 2 3
 *   4 5 6
 *   7 8 9
 *     0 ⌫
 *
 * Bottom-left blank — empty View (not Touchable) per design § 4.3 a11y
 * spec, prevents stray accessibility focus. Future biometric prompt
 * integration (M4.1 UnlockScreen) можно added через prop slot если
 * required — defer per CLAUDE.md «не add features that the task
 * requires».
 *
 * Disabled state — opacity-50 + pointerEvents='none' (mirrors Phase 3
 * Button precedent for `isDisabled`). Used by CreatePin (isHashing) +
 * ConfirmPin (isCommitting) per design § 4.3 line 326 + § 4.4 line 337.
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 2 line 150 (M2.3 deliverable)
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.3 line 325 (a11y spec)
 */

import React from 'react';
import { TouchableOpacity, View, Text } from 'react-native';
import clsx from 'clsx';

interface PinPadProps {
  onPressDigit: (digit: string) => void;
  onPressBackspace: () => void;
  disabled?: boolean;
}

const ROWS: readonly (readonly string[])[] = [
  ['1', '2', '3'],
  ['4', '5', '6'],
  ['7', '8', '9'],
  ['', '0', '⌫'],
] as const;

export function PinPad({
  onPressDigit,
  onPressBackspace,
  disabled = false,
}: PinPadProps) {
  return (
    <View
      pointerEvents={disabled ? 'none' : 'auto'}
      className={clsx('w-full', disabled && 'opacity-50')}
    >
      {ROWS.map((row, rowIdx) => (
        <View key={rowIdx} className="flex-row justify-around mb-3">
          {row.map((cell, colIdx) => {
            if (cell === '') {
              return <View key={colIdx} className="w-20 h-20" />;
            }
            const isBackspace = cell === '⌫';
            const handlePress = isBackspace
              ? onPressBackspace
              : () => onPressDigit(cell);
            const accessibilityLabel = isBackspace
              ? 'Backspace'
              : `Digit ${cell}`;
            return (
              <TouchableOpacity
                key={colIdx}
                onPress={handlePress}
                accessibilityRole="button"
                accessibilityLabel={accessibilityLabel}
                className="w-20 h-20 rounded-full items-center justify-center bg-canvas border border-accent-periwinkle/30"
              >
                <Text className="text-2xl text-accent-periwinkle font-medium">
                  {cell}
                </Text>
              </TouchableOpacity>
            );
          })}
        </View>
      ))}
    </View>
  );
}
