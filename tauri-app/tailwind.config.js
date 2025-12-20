/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        dark: {
          50: '#f8fafc',
          100: '#f1f5f9',
          200: '#e2e8f0',
          300: '#cbd5e1',
          400: '#94a3b8',
          500: '#64748b',
          600: '#475569',
          700: '#334155',
          800: '#1e293b',
          850: '#161b22',
          900: '#0f172a',
          950: '#0d1117',
        },
        accent: {
          green: '#10b981',
          yellow: '#eab308',
          orange: '#f97316',
          red: '#ef4444',
          blue: '#3b82f6',
        }
      },
      fontFamily: {
        'mono': ['JetBrains Mono', 'Consolas', 'Monaco', 'Courier New', 'monospace'],
      }
    },
  },
  plugins: [],
}