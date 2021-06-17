module.exports = {
  // Generated from https://eslintconfig.dev/cra/typescript-prettier
  extends: [
    "react-app", // Create React App base settings
    "eslint:recommended", // recommended ESLint rules
    "plugin:@typescript-eslint/recommended", // recommended rules from @typescript-eslint/eslint-plugin
    "prettier", // Uses eslint-config-prettier to disable ESLint rules from @typescript-eslint/eslint-plugin that would conflict with Prettier. Now part of "prettier"
    "plugin:prettier/recommended", // Enables eslint-plugin-prettier and eslint-config-prettier. This will display Prettier errors as ESLint errors. Make sure this is always the last configuration in the extends array.
  ],
  rules: {
    "@typescript-eslint/no-explicit-any": "off",
    "@typescript-eslint/explicit-module-boundary-types": [
      "warn",
      {
        allowArgumentsExplicitlyTypedAsAny: true,
      },
    ],
    "@typescript-eslint/no-empty-function": "off",
  },
};
