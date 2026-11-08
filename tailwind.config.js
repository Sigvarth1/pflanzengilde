/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Phantom-inspired accent: lavender violet (was plant green)
        forest: {
          50: '#f6f4ff',
          100: '#ece8fe',
          200: '#dcd5fc',
          300: '#c5baf8',
          400: '#ab9ff2',
          500: '#9282ea',
          600: '#6e5bd6',
          700: '#5b48bf',
          800: '#4b3c9e',
          900: '#3f3480',
          950: '#2a2252',
        },
        // Lavender-tinted neutrals replace warm stone everywhere
        stone: {
          50: '#fbfaff',
          100: '#f5f2ff',
          200: '#e8e3f8',
          300: '#d3cbee',
          400: '#a49ac6',
          500: '#716693',
          600: '#5d5383',
          700: '#4e4475',
          800: '#443a69',
          900: '#3c315b',
          950: '#221a38',
        },
        cream: '#fffdf8',
        earth: {
          50: '#fdfbf7',
          100: '#f7f2ea',
          200: '#ede2d1',
          300: '#ddc9ad',
          400: '#c7a783',
          500: '#b68e64',
          600: '#a37651',
          700: '#875d42',
          800: '#6f4c39',
          900: '#5c3f32',
          950: '#322019',
        }
      },
      fontFamily: {
        sans: ['"Figtree Variable"', '-apple-system', 'BlinkMacSystemFont', '"Segoe UI"', 'Roboto', 'Helvetica', 'Arial', 'sans-serif'],
      },
      // Softer weights: large type carries hierarchy, not heavy strokes
      fontWeight: {
        semibold: '560',
        bold: '620',
        extrabold: '660',
        black: '720',
      },
      borderRadius: {
        lg: '0.75rem',
        xl: '1rem',
        '2xl': '1.5rem',
        '3xl': '2rem',
      },
    },
  },
  plugins: [],
}
