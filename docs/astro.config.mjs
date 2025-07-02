// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
  integrations: [
    starlight({
      title: 'SciWIn Client',
      customCss: [
        '@fontsource/fira-sans/400.css',
        '@fontsource/fira-sans/700.css',
        '@fontsource/fira-sans/900.css',
        '@fontsource/fira-sans/400-italic.css',
        '@fontsource/fira-sans/700-italic.css',
        '@fontsource/fira-sans/900-italic.css',
        './src/styles/global.css'
      ],
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/fairagro/sciwin_client' }],
      components: {
        Hero: './src/components/Hero.astro',
      }
    }),
  ],

  vite: {
    plugins: [tailwindcss()],
  },
});