/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        void: "#000e1a",
        primary: "#0088ff",
        alert: "#ff0000",
        warning: "#eeff00",
        success: "#1aff00",
        accent: "#ff00e6",
      },
      fontFamily: {
        mono: ["JetBrains Mono", "Fira Code", "monospace"],
      },
    },
  },
  plugins: [],
};
