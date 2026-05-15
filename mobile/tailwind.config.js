/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./App.{js,jsx,ts,tsx}', './src/**/*.{js,jsx,ts,tsx}'],
  presets: [require('nativewind/preset')],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        canvas: 'rgb(var(--color-canvas) / <alpha-value>)',
        brand: {
          deep: 'rgb(var(--color-brand-deep) / <alpha-value>)',
        },
        surface: {
          alt: 'rgb(var(--color-surface-alt) / <alpha-value>)',
          border: 'rgb(var(--color-surface-border) / <alpha-value>)',
          card: 'rgb(var(--color-surface-card) / <alpha-value>)',
          elevated: 'rgb(var(--color-surface-elevated) / <alpha-value>)',
        },
        ink: {
          primary: 'rgb(var(--color-ink-primary) / <alpha-value>)',
          muted: 'rgb(var(--color-ink-muted) / <alpha-value>)',
        },
        neutral: {
          mid: 'rgb(var(--color-neutral-mid) / <alpha-value>)',
          soft: 'rgb(var(--color-neutral-soft) / <alpha-value>)',
        },
        accent: {
          periwinkle: 'rgb(var(--color-accent-periwinkle) / <alpha-value>)',
          deep: 'rgb(var(--color-accent-deep) / <alpha-value>)',
          soft: 'rgb(var(--color-accent-soft) / <alpha-value>)',
        },
        semantic: {
          success: 'rgb(var(--color-semantic-success) / <alpha-value>)',
          warn: 'rgb(var(--color-semantic-warn) / <alpha-value>)',
          danger: 'rgb(var(--color-semantic-danger) / <alpha-value>)',
        },
      },
      borderRadius: {
        'rw-sm': '10px',
        'rw-md': '14px',
        'rw-lg': '18px',
        'rw-xl': '24px',
        'rw-pill': '9999px',
      },
    },
  },
  plugins: [],
};
