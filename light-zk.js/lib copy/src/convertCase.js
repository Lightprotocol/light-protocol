"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.isCamelCase = exports.snakeCaseToCamelCase = exports.camelToScreamingSnake = exports.toCamelCase = exports.toSnakeCase = void 0;
function toSnakeCase(str) {
    // Convert kebab-case to snake_case
    str = str.replace(/-/g, "_");
    // Convert camelCase to snake_case
    return str
        .replace(/([a-z0-9]|(?=[A-Z]))([A-Z])/g, "$1_$2") // Match cases like: aB or 0B or (boundary of a capital letter)B
        .toLowerCase();
}
exports.toSnakeCase = toSnakeCase;
function toCamelCase(input) {
    return input
        .split(/[_-]/)
        .map((word, index) => {
        if (index === 0) {
            return word.toLowerCase();
        }
        return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase();
    })
        .join("");
}
exports.toCamelCase = toCamelCase;
function camelToScreamingSnake(str) {
    return str
        .replace(/([a-z0-9])([A-Z])/g, "$1_$2") // Insert underscore between a lowercase followed by an uppercase
        .toUpperCase();
}
exports.camelToScreamingSnake = camelToScreamingSnake;
const snakeCaseToCamelCase = (str, uppercaseFirstLetter = false) => str
    .split("_")
    .reduce((res, word, i) => i === 0 && !uppercaseFirstLetter
    ? word.toLowerCase()
    : `${res}${word.charAt(0).toUpperCase()}${word
        .substr(1)
        .toLowerCase()}`, "");
exports.snakeCaseToCamelCase = snakeCaseToCamelCase;
function isCamelCase(str) {
    return /^[a-z]+([A-Z][a-z0-9]*)*$/.test(str);
}
exports.isCamelCase = isCamelCase;
//# sourceMappingURL=convertCase.js.map