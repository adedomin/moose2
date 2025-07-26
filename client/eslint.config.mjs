import { defineConfig, globalIgnores } from "eslint/config";
import globals from "globals";
import path from "node:path";
import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import { FlatCompat } from "@eslint/eslintrc";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
    baseDirectory: __dirname,
    recommendedConfig: js.configs.recommended,
    allConfig: js.configs.all
});

export default defineConfig([globalIgnores(["**/*.bundle.js"]), {
    extends: compat.extends("eslint:recommended"),

    languageOptions: {
        globals: {
            ...globals.browser,
        },

        ecmaVersion: 2022,
        sourceType: "module",
    },

    rules: {
        strict: ["error", "global"],
        indent: ["warn", 2],
        "linebreak-style": ["warn", "unix"],
        "no-trailing-spaces": ["warn"],

        quotes: ["error", "single", {
            avoidEscape: true,
        }],

        semi: ["error", "always"],

        "no-console": ["warn", {
            allow: ["error"],
        }],

        "object-shorthand": ["error"],
        "quote-props": ["error", "as-needed"],
        "array-callback-return": ["error"],
        "prefer-rest-params": ["error"],
        "prefer-spread": ["error"],

        "prefer-destructuring": ["error", {
            AssignmentExpression: {
                object: false,
                array: false,
            },
        }],

        "brace-style": ["error", "stroustrup"],
        "spaced-comment": ["error"],
        "space-before-blocks": ["error"],
        "keyword-spacing": ["error"],
        "comma-dangle": ["error", "always-multiline"],
        "no-var": ["warn"],
    },
}]);