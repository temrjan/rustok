/**
 * @format
 */

// M4 C1 attempt: import Worklets at the entry point (before any
// `App.tsx` evaluation) so the native bridge is initialized before
// the React tree mounts. Earlier attempt #6 (incident doc) added the
// import inside App.tsx and had no effect; entry-point timing differs.
import 'react-native-worklets';

import { AppRegistry } from 'react-native';
import App from './App';
import { name as appName } from './app.json';

AppRegistry.registerComponent(appName, () => App);
