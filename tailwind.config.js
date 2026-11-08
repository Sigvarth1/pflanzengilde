/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Moss / leaf green accent
        forest: {
          50: '#f3f6ef',
          100: '#e4ecdb',
          200: '#c9d9b8',
          300: '#a8c08f',
          400: '#86a66a',
          500: '#6a8c4f',
          600: '#52713c',
          700: '#415a31',
          800: '#36492a',
          900: '#2e3d25',
          950: '#172112',
        },
        // Warm soil neutrals
        stone: {
          50: '#fbfaf6',
          100: '#f4f1ea',
          200: '#e7e1d5',
          300: '#d3cab8',
          400: '#a89e8a',
          500: '#786e5d',
          600: '#635a4b',
          700: '#4f473b',
          800: '#3d362d',
          900: '#2e2922',
          950: '#1c1814',
        },
        // Semantic "good / suits": a warmer leaf green than stock emerald
        emerald: {
          50: '#eef6ec',
          100: '#d9ebd4',
          200: '#b6d7ad',
          300: '#8cbd80',
          400: '#67a25b',
          500: '#4c8a42',
          600: '#3b7134',
          700: '#305b2c',
          800: '#294a26',
          900: '#233e21',
          950: '#10220f',
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
        lg: '0.625rem',
        xl: '0.875rem',
        '2xl': '1.25rem',
        '3xl': '1.5rem',
      },
    },
  },
  plugins: [],
}
