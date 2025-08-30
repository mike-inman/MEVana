/* ui-web/.eslintrc.cjs */
module.exports = {
    root: true,
    parser: '@typescript-eslint/parser',
    parserOptions: { ecmaVersion: 'latest', sourceType: 'module' },
    settings: { react: { version: 'detect' } },
    plugins: ['@typescript-eslint', 'react', 'react-hooks', 'jsx-a11y', 'import'],
    extends: [
      'eslint:recommended',
      'plugin:@typescript-eslint/recommended',
      'plugin:react/recommended',
      'plugin:react-hooks/recommended',
      'plugin:jsx-a11y/recommended',
      'plugin:import/typescript',
      'prettier'
    ],
    rules: {
      'react/react-in-jsx-scope': 'off', // Vite/React 17+
      'import/order': [
        'warn',
        {
          groups: [['builtin','external'], ['internal'], ['parent','sibling','index']],
          'newlines-between': 'always',
          alphabetize: { order: 'asc', caseInsensitive: true }
        }
      ],
      '@typescript-eslint/explicit-module-boundary-types': 'off',
      '@typescript-eslint/no-explicit-any': 'off', // relax early, tighten later
      'no-console': ['warn', { allow: ['warn','error'] }],
    }
  }
  