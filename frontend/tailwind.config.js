/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./index.html",
        "./src/**/*.{js,ts,jsx,tsx}",
    ],
    theme: {
        extend: {
            // Same values as the CSS variables in src/index.css
            colors: {
                ground: '#0F3B2E',
                ink: '#F3EAD7',
                accent: '#F2B544',
            },
            fontFamily: {
                sans: ['"Instrument Sans"', 'system-ui', 'sans-serif'],
                serif: ['"Instrument Serif"', 'Georgia', 'serif'],
                mono: ['"Geist Mono"', 'ui-monospace', 'monospace'],
            },
        },
    },
    plugins: [],
}
