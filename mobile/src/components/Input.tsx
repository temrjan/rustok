/**
 * Input — Phase 3 M2 Commit 1.
 *
 * Text input with optional label and error message. Border swaps to
 * semantic-danger on error. Placeholder color is resolved via the
 * tokens module (placeholderTextColor accepts a value, not className).
 */

import React from 'react';
import {
  Text,
  TextInput,
  View,
  type TextInputProps,
} from 'react-native';
import { cva } from 'class-variance-authority';
import { useColorScheme } from 'nativewind';
import { palette } from '../theme/tokens';

const inputVariants = cva(
  'rounded-md border px-3 py-2 text-ink-primary text-base',
  {
    variants: {
      state: {
        normal: 'border-ink-muted',
        error: 'border-semantic-danger',
      },
    },
    defaultVariants: {
      state: 'normal',
    },
  },
);

interface InputProps {
  value: string;
  onChangeText: (value: string) => void;
  placeholder?: string;
  secureTextEntry?: boolean;
  error?: string;
  /**
   * Visible label rendered above the field. Doubles as the screen-reader
   * label when `accessibilityLabel` is omitted.
   *
   * @remarks One of `label` or `accessibilityLabel` MUST be provided.
   */
  label?: string;
  accessibilityLabel?: string;
  autoCapitalize?: TextInputProps['autoCapitalize'];
  autoCorrect?: boolean;
}

export function Input({
  value,
  onChangeText,
  placeholder,
  secureTextEntry,
  error,
  label,
  accessibilityLabel,
  autoCapitalize,
  autoCorrect,
}: InputProps) {
  if (__DEV__ && label === undefined && accessibilityLabel === undefined) {
    console.warn(
      '[Input] either `label` or `accessibilityLabel` must be set ' +
        '(C1 a11y constraint).',
    );
  }
  const { colorScheme } = useColorScheme();
  const hasError = error !== undefined && error.length > 0;
  const placeholderColor =
    colorScheme === 'dark'
      ? palette.dark.ink.muted
      : palette.light.ink.muted;

  return (
    <View className="mb-2">
      {label !== undefined && (
        <Text className="text-ink-muted text-xs mb-1">{label}</Text>
      )}
      <TextInput
        value={value}
        onChangeText={onChangeText}
        placeholder={placeholder}
        placeholderTextColor={placeholderColor}
        secureTextEntry={secureTextEntry}
        autoCapitalize={autoCapitalize}
        autoCorrect={autoCorrect}
        accessibilityLabel={accessibilityLabel ?? label}
        className={inputVariants({ state: hasError ? 'error' : 'normal' })}
      />
      {hasError && (
        <Text className="text-semantic-danger text-xs mt-1">{error}</Text>
      )}
    </View>
  );
}
