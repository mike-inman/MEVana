import type { Config } from 'tailwindcss'
export default {
  darkMode: ['class'],
  content: ['./index.html','./src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: 'var(--bg)', panel: 'var(--panel)', panel2: 'var(--panel-2)',
        text: 'var(--text)', muted: 'var(--muted)',
        accent: 'var(--accent)', accent2: 'var(--accent-2)',
        up: 'var(--up)', down: 'var(--down)', warn: 'var(--warn)'
      },
      boxShadow: { neon: '0 0 0 1px var(--accent) inset, var(--shadow)' },
      borderRadius: { xl2: '1rem' }
    }
  },
  plugins: []
} satisfies Config