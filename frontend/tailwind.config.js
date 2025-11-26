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
          bg: "#0d1117",
          surface: "#161b22",
          elevated: "#1c2128",
          border: "#30363d",
          borderSubtle: "#21262d",
        },
        accent: {
          blue: "#388bfd",
          blueHover: "#58a6ff",
          green: "#3fb950",
          yellow: "#d29922",
          orange: "#db6d28",
          red: "#f85149",
        },
        text: {
          primary: "#e6edf3",
          secondary: "#8b949e",
          muted: "#6e7681",
        },
      },
      borderRadius: {
        sm: "4px",
        DEFAULT: "6px",
      },
      fontSize: {
        xs: ["13px", "19px"],
        sm: ["14px", "21px"],
        base: ["15px", "23px"],
        lg: ["17px", "25px"],
      },
    },
  },
  plugins: [],
}
