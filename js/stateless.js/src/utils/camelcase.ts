/**
 * Manual implementation of camelCase functionality
 * Replaces camelcase and camelcase-keys packages
 */

/**
 * Convert a string to camelCase
 */
export function camelCase(str: string): string {
    if (!str || typeof str !== 'string') {
        return str;
    }

    // Handle already camelCase strings
    if (!/[_.\-\s]/.test(str) && str === str.toLowerCase()) {
        return str;
    }

    return (
        str
            // Replace special characters with spaces
            .replace(/[_.\-\s]+(.)?/g, (_, chr) =>
                chr ? chr.toUpperCase() : '',
            )
            // Ensure first character is lowercase
            .replace(/^[A-Z]/, chr => chr.toLowerCase())
    );
}

/**
 * Check if value is a plain object
 */
function isObject(value: any): boolean {
    return (
        typeof value === 'object' &&
        value !== null &&
        !(value instanceof RegExp) &&
        !(value instanceof Error) &&
        !(value instanceof Date) &&
        !(value instanceof Buffer) &&
        !Array.isArray(value)
    );
}

/**
 * Convert object keys to camelCase
 */
export function camelcaseKeys<T = any>(
    input: T,
    options: { deep?: boolean } = {},
): T {
    const { deep = false } = options;

    if (!isObject(input)) {
        return input;
    }

    const result: any = {};

    for (const [key, value] of Object.entries(input as any)) {
        const camelKey = camelCase(key);

        if (deep && isObject(value)) {
            result[camelKey] = camelcaseKeys(value, options);
        } else if (deep && Array.isArray(value)) {
            result[camelKey] = value.map(item =>
                isObject(item) ? camelcaseKeys(item, options) : item,
            );
        } else {
            result[camelKey] = value;
        }
    }

    return result as T;
}
