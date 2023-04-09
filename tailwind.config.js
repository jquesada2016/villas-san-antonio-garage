/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "index.html",
    "src/**/*.ts",
    "wasm/**/*.rs",
    "wasm/daisyui_component_classes.txt",
  ],
  theme: {
    extend: {},
  },
  plugins: [require("daisyui")],
};
