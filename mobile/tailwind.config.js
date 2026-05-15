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
    },
  },
  plugins: [],
};
