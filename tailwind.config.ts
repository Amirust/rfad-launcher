import type { Config } from 'tailwindcss';

export default {
  content: [
    './components/**/*.{js,vue,ts}',
    './layouts/**/*.vue',
    './pages/**/*.vue',
    './plugins/**/*.{js,ts}',
    './app.vue',
    './error.vue'
  ],
  theme: {
    extend: {
      padding: {
        '18': '72px'
      },
      screens: {
        // => @media (min-width: 1860px) { ... }
        '3xl': '1860px',
        // => @media (min-width: 1988px) { ... }
        '2k': '1988px',
        // => @media (min-width: 2500px) { ... }
        '4k': '2500px'
      },
      fontFamily: {
        sans: [ 'Futura', 'sans-serif' ]
      },
      colors: {
        primary: '#FFEABF',
        primaryHalf: '#FFEABF80',
        secondary: '#BFAF8F',
        secondaryDarker: '#867A64',
        secondaryHalf: '#BFAF8F80',
        'primary-25': '#FFEABF40',
        'secondary-25': '#BFAF8F40',
        block: '#0D0C0A',
        blockTransparent: '#0d0c0ae6',
        blockBorder: '#202020',
        error: '#D42E4F'
      }
    }
  },
  plugins: []
} as Config;
