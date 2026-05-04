/**
 * Components barrel — Phase 3 M2.
 *
 * Public re-exports for the mobile design-system primitives.
 * Import sites use: `import { Button, Input } from '../components';`
 */

export { Button } from './Button';
export { Input } from './Input';
// TODO M4: re-export Modal once Reanimated 4 / Worklets native bridge
// initializes correctly. Temporarily disabled to keep the bundle from
// eagerly evaluating @gorhom/bottom-sheet, which throws WorkletsError
// at module load time. Modal source remains in src/components/Modal.tsx
// for restoration.
// export { Modal } from './Modal';
export { PageHeader } from './PageHeader';
export { Spinner } from './Spinner';
export { Switch } from './Switch';
export { ThemeSwitcher } from './ThemeSwitcher';
export { toast, ToastProvider } from './Toast';
