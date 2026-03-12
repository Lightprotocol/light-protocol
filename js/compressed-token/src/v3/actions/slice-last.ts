export function sliceLast<T>(items: T[]): { rest: T[]; last: T } {
    if (items.length === 0) {
        throw new Error('sliceLast: array must not be empty');
    }
    return { rest: items.slice(0, -1), last: items.at(-1)! };
}
