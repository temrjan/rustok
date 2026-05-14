/**
 * Manual mock for `react-native-svg` (auto-loaded by Jest via the
 * `__mocks__/<package>` adjacent-to-`node_modules` convention).
 *
 * The real library registers a native view manager at import time; outside a
 * real RN runtime this throws — same pattern as sibling mocks
 * (`react-native-mmkv`, `react-native-fs`, etc.).
 *
 * ## Why a string mock works
 *
 * React's test renderer accepts string-valued component types as opaque host
 * components — `React.createElement('Svg', { ... }, children)` produces a
 * tree without invoking any native bridge. `lucide-react-native` builds
 * icons from these primitives, so mocking `react-native-svg` is sufficient
 * to let icons render in Jest without a separate `lucide-react-native`
 * mock.
 *
 * Add new primitives here as future call sites pull them in.
 */

export const Svg = 'Svg';
export const G = 'G';
export const Path = 'Path';
export const Circle = 'Circle';
export const Rect = 'Rect';
export const Ellipse = 'Ellipse';
export const Line = 'Line';
export const Polygon = 'Polygon';
export const Polyline = 'Polyline';
export const Mask = 'Mask';
export const ClipPath = 'ClipPath';
export const Defs = 'Defs';
export const LinearGradient = 'LinearGradient';
export const RadialGradient = 'RadialGradient';
export const Stop = 'Stop';
export const Text = 'SvgText';
export const TSpan = 'TSpan';
export const Use = 'Use';
export const Symbol = 'Symbol';
export const Image = 'SvgImage';
export const Pattern = 'Pattern';

// `SvgXml` renders a raw SVG document string. Phase 5 M3a uses it for
// the QR code returned from the Rust bridge (`getWalletQrSvg`).
export const SvgXml = 'SvgXml';
export const SvgUri = 'SvgUri';
export const SvgCss = 'SvgCss';
export const SvgCssUri = 'SvgCssUri';

export default Svg;
