const js = require("@eslint/js");
const tseslint = require("@typescript-eslint/eslint-plugin");
const tsParser = require("@typescript-eslint/parser");
const prettier = require("eslint-config-prettier");

module.exports = [
  {
    ignores: ["node_modules/**", "dist/**", "bin/**", "accounts/**", "test/**", "eslint.config.js"],
  },
  js.configs.recommended,
  prettier,
  {
    files: ["**/*.js"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "commonjs",
      globals: {
        require: "readonly",
        module: "readonly",
        process: "readonly",
        __dirname: "readonly",
        __filename: "readonly",
        exports: "readonly",
        console: "readonly",
      },
    },
  },
  {
    files: ["scripts/**/*.ts"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: "module",
      },
      globals: {
        process: "readonly",
        console: "readonly",
        __dirname: "readonly",
        __filename: "readonly",
        require: "readonly",
      },
    },
    plugins: {
      "@typescript-eslint": tseslint,
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      "@typescript-eslint/ban-ts-comment": 0,
      "@typescript-eslint/no-explicit-any": 0,
      "@typescript-eslint/no-var-requires": 0,
      "@typescript-eslint/no-require-imports": 0,
      "@typescript-eslint/no-unused-vars": 0,
      "no-undef": 0,
    },
  },
  {
    files: ["src/**/*.ts", "src/**/*.tsx"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        project: ["./tsconfig.json"],
        ecmaVersion: 2022,
        sourceType: "module",
      },
      globals: {
        process: "readonly",
        console: "readonly",
        __dirname: "readonly",
        __filename: "readonly",
      },
    },
    plugins: {
      "@typescript-eslint": tseslint,
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      "@typescript-eslint/ban-ts-comment": 0,
      "@typescript-eslint/no-explicit-any": 0,
      "@typescript-eslint/no-var-requires": 0,
      "@typescript-eslint/no-unused-vars": 0,
      "no-prototype-builtins": 0,
      "no-undef": 0, // TypeScript handles this
    },
  },
];