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
          bg: "#1b2636",
          surface: "#2a3f5f",
          border: "#3d5a80",
        },
      },
    },
  },
  plugins: [],
}
