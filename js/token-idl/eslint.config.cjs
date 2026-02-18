const js = require('@eslint/js');
const tseslint = require('@typescript-eslint/eslint-plugin');
const tsParser = require('@typescript-eslint/parser');

module.exports = [
    {
        ignores: [
            'node_modules/**',
            'dist/**',
            'build/**',
            '*.config.js',
            'eslint.config.js',
            'eslint.config.cjs',
        ],
    },
    js.configs.recommended,
    {
        files: ['**/*.js', '**/*.cjs', '**/*.mjs'],
        languageOptions: {
            ecmaVersion: 2022,
            sourceType: 'module',
            globals: {
                require: 'readonly',
                module: 'readonly',
                process: 'readonly',
                __dirname: 'readonly',
                __filename: 'readonly',
                exports: 'readonly',
                console: 'readonly',
                Buffer: 'readonly',
            },
        },
    },
    {
        files: ['src/**/*.ts', 'scripts/**/*.ts'],
        languageOptions: {
            parser: tsParser,
            parserOptions: {
                ecmaVersion: 2022,
                sourceType: 'module',
            },
            globals: {
                process: 'readonly',
                console: 'readonly',
                __dirname: 'readonly',
                __filename: 'readonly',
                Buffer: 'readonly',
            },
        },
        plugins: {
            '@typescript-eslint': tseslint,
        },
        rules: {
            ...tseslint.configs.recommended.rules,
            '@typescript-eslint/ban-ts-comment': 0,
            '@typescript-eslint/no-explicit-any': 0,
            '@typescript-eslint/no-var-requires': 0,
            '@typescript-eslint/no-unused-vars': 0,
            '@typescript-eslint/no-require-imports': 0,
            'no-prototype-builtins': 0,
            'no-undef': 0,
            'no-unused-vars': 0,
            'no-redeclare': 0,
        },
    },
];
