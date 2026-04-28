/**
 * Rustok Wallet — mobile UI shell.
 *
 * M2 scaffold from docs/POC-FOUNDATION.md (Phase 1).
 * The "Generate mnemonic" button is a placeholder; the actual call into
 * rustok-mobile-bindings via uniffi-bindgen-react-native will be wired
 * in Milestone 4.
 */

import { useState } from 'react';
import {
  StatusBar,
  StyleSheet,
  Text,
  TouchableOpacity,
  useColorScheme,
  View,
} from 'react-native';
import {
  SafeAreaProvider,
  useSafeAreaInsets,
} from 'react-native-safe-area-context';

function App() {
  const isDarkMode = useColorScheme() === 'dark';
  return (
    <SafeAreaProvider>
      <StatusBar barStyle={isDarkMode ? 'light-content' : 'dark-content'} />
      <AppContent isDarkMode={isDarkMode} />
    </SafeAreaProvider>
  );
}

function AppContent({ isDarkMode }: { isDarkMode: boolean }) {
  const insets = useSafeAreaInsets();
  const [phrase, setPhrase] = useState<string | null>(null);

  const onGenerate = () => {
    // Wired to rustok-mobile-bindings in Milestone 4.
    setPhrase('uniffi bridge will be wired in Milestone 4');
  };

  const containerStyle = [
    styles.container,
    isDarkMode ? styles.containerDark : styles.containerLight,
    { paddingTop: insets.top, paddingBottom: insets.bottom },
  ];
  const titleStyle = [
    styles.title,
    isDarkMode ? styles.textPrimaryDark : styles.textPrimaryLight,
  ];
  const subtitleStyle = [
    styles.subtitle,
    isDarkMode ? styles.textMutedDark : styles.textMutedLight,
  ];
  const phraseStyle = [
    styles.phrase,
    isDarkMode ? styles.textMutedDark : styles.textMutedLight,
  ];

  return (
    <View style={containerStyle}>
      <Text style={titleStyle}>Rustok</Text>
      <Text style={subtitleStyle}>Phase 1 — POC Foundation</Text>

      <TouchableOpacity style={styles.button} onPress={onGenerate}>
        <Text style={styles.buttonText}>Generate mnemonic</Text>
      </TouchableOpacity>

      {phrase !== null && <Text style={phraseStyle}>{phrase}</Text>}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    paddingHorizontal: 24,
    alignItems: 'center',
    justifyContent: 'center',
  },
  containerLight: {
    backgroundColor: '#FFFFFF',
  },
  containerDark: {
    backgroundColor: '#0A1123',
  },
  title: {
    fontSize: 32,
    fontWeight: '700',
    marginBottom: 8,
  },
  subtitle: {
    fontSize: 14,
    marginBottom: 32,
  },
  textPrimaryLight: {
    color: '#0A1123',
  },
  textPrimaryDark: {
    color: '#FFFFFF',
  },
  textMutedLight: {
    color: '#3A3E6C',
  },
  textMutedDark: {
    color: '#8A8CAC',
  },
  button: {
    backgroundColor: '#8387C3',
    paddingHorizontal: 24,
    paddingVertical: 12,
    borderRadius: 12,
  },
  buttonText: {
    color: '#FFFFFF',
    fontSize: 16,
    fontWeight: '600',
  },
  phrase: {
    marginTop: 24,
    fontSize: 14,
    textAlign: 'center',
  },
});

export default App;
