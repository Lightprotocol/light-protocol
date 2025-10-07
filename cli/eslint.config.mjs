// @ts-check
import eslint from '@eslint/js';

export default [
  // Ignore patterns (replaces .eslintignore)
  {
    ignores: [
      'node_modules/**',
      'dist/**',
      'bin/**',
      'accounts/**',
      'test/**',
      'test_bin/**',
      'scripts/**',
    ],
  },

  // Base ESLint recommended rules
  eslint.configs.recommended,

  // Custom rules
  {
    rules: {
      'no-prototype-builtins': 0,
    },
  },
];
