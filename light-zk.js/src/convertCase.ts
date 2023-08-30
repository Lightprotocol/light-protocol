export function toSnakeCase(str: string): string {
  // Convert kebab-case to snake_case
  str = str.replace(/-/g, "_");

  // Convert camelCase to snake_case
  return str
    .replace(/([a-z0-9]|(?=[A-Z]))([A-Z])/g, "$1_$2") // Match cases like: aB or 0B or (boundary of a capital letter)B
    .toLowerCase();
}

export function toCamelCase(input: string): string {
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

export function camelToScreamingSnake(str: string) {
  return str
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2") // Insert underscore between a lowercase followed by an uppercase
    .toUpperCase();
}

export const snakeCaseToCamelCase = (
  str: string,
  uppercaseFirstLetter: boolean = false,
) =>
  str
    .split("_")
    .reduce(
      (res, word, i) =>
        i === 0 && !uppercaseFirstLetter
          ? word.toLowerCase()
          : `${res}${word.charAt(0).toUpperCase()}${word
              .substr(1)
              .toLowerCase()}`,
      "",
    );

export function isCamelCase(str: string): boolean {
  return /^[a-z]+([A-Z][a-z0-9]*)*$/.test(str);
}
