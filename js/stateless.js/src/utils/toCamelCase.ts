export function toCamelCase(
  obj: Array<any> | Object | any
): Array<any> | Object | any {
  if (Array.isArray(obj)) {
    return obj.map((v) => toCamelCase(v));
  } else if (obj !== null && obj.constructor === Object) {
    return Object.keys(obj).reduce((result, key) => {
      const camelCaseKey = key.replace(/([-_][a-z])/gi, ($1) => {
        return $1.toUpperCase().replace("-", "").replace("_", "");
      });
      result[camelCaseKey] = toCamelCase(obj[key]);
      return result;
    }, {} as any);
  }
  return obj;
}

//@ts-ignore
if (import.meta.vitest) {
  //@ts-ignore
  const { it, expect, describe } = import.meta.vitest;

  describe("toCamelCase", () => {
    it("should convert object keys to camelCase", () => {
      const input = { test_key: 1, "another-testKey": 2 };
      const expected = { testKey: 1, anotherTestKey: 2 };
      expect(toCamelCase(input)).toEqual(expected);
    });

    it("should handle arrays of objects", () => {
      const input = [{ array_key: 3 }, { "another_array-key": 4 }];
      const expected = [{ arrayKey: 3 }, { anotherArrayKey: 4 }];
      expect(toCamelCase(input)).toEqual(expected);
    });

    it("should return the input if it is neither an object nor an array", () => {
      const input = "testString";
      expect(toCamelCase(input)).toBe(input);
    });
  });
}
