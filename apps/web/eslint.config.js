import js from '@eslint/js';
import globals from 'globals';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import tseslint from 'typescript-eslint';
import importPlugin from 'eslint-plugin-import';
import { defineConfig, globalIgnores } from 'eslint/config';
import { readdirSync } from 'node:fs';

// Feature boundaries (bulletproof-react): shared -> features -> app, and no feature imports another feature.
const features = readdirSync(new URL('./src/features', import.meta.url), { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name);

const featureZones = features.map((name) => ({
  target: `./src/features/${name}`,
  from: './src/features',
  except: [`./${name}`],
  message: `Feature "${name}" must not import from another feature. Compose features in src/app/routes.`,
}));

export default defineConfig([
  globalIgnores(['dist', 'public/**', 'src/types/api/**']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [js.configs.recommended, tseslint.configs.recommended, reactHooks.configs.flat.recommended, reactRefresh.configs.vite],
    languageOptions: { ecmaVersion: 2022, globals: globals.browser },
    plugins: { import: importPlugin },
    settings: { 'import/resolver': { typescript: { project: './tsconfig.app.json' } } },
    rules: {
      'import/no-restricted-paths': [
        'error',
        {
          zones: [
            ...featureZones,
            { target: './src/features', from: './src/app', message: 'Features must not import from the app layer.' },
            {
              target: ['./src/components', './src/hooks', './src/lib', './src/stores', './src/types', './src/utils', './src/config'],
              from: ['./src/features', './src/app'],
              message: 'Shared code must not import from features or app.',
            },
          ],
        },
      ],
      // Context and query-key files legitimately export more than components.
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
    },
  },
  { files: ['**/*.test.{ts,tsx}', 'src/testing/**'], rules: { 'react-refresh/only-export-components': 'off' } },
  { files: ['*.config.{js,ts}'], languageOptions: { globals: globals.node } },
]);
