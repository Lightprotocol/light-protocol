export const toArray = <T>(value: T | T[]) =>
  Array.isArray(value) ? value : [value];
