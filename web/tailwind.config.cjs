module.exports = {
  purge: [],
  darkMode: "class", // or 'media' or 'class'
  theme: {
    extend: {
      colors: {},
      textColor: {
        skin: {
          bg: "var(--color-bg)",
          base: "var(--color-text-base)",
          muted: "var(--color-text-muted)",
          button: "var(--color-text-button)",
          input: "var(--color-text-input)",
        },
      },
      backgroundColor: {
        skin: {
          fill: "var(--color-fill)",
          "button-accent": "var(--color-button-accent)",
          "button-hover": "var(--color-button-hover)",
          card: "var(--color-card-bg)",
          input: "var(--color-input-bg)",
        },
      },
      borderColor: {
        skin: {
          divider: "var(--color-divider)",
          input: "var(--color-input-border)",
        },
      },
    },
  },
  variants: {
    extend: {},
  },
  plugins: [require("@tailwindcss/forms")],
};
