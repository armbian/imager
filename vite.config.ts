import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig(({ command, mode }) => ({
  plugins: [react()],
  define: {
    // The dev scenarios panel ships in `vite` and `build:dev` only; Vite 8 keeps DEV false for any build.
    __DEV_SCENARIOS__: JSON.stringify(command === 'serve' || mode === 'development'),
  },
}))
