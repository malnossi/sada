import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  onwarn: (warning, handler) => {
    // Suppress all accessibility (a11y) warnings in build outputs
    if (warning.code.startsWith("a11y_") || warning.code.startsWith("a11y-")) {
      return;
    }
    handler(warning);
  }
};
