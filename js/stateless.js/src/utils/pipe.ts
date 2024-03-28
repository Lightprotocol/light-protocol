/** pipe function */
export function pipe<T, R>(
    initialFunction: (arg: T) => R,
    ...functions: ((arg: R) => R)[]
): (initialValue: T) => R {
    return (initialValue: T): R =>
        functions.reduce(
            (currentValue, currentFunction) => currentFunction(currentValue),
            initialFunction(initialValue),
        );
}

//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { it, expect, describe } = import.meta.vitest;

    describe('pipe', () => {
        it('should return the result of applying all fns to the initial value', () => {
            const addOne = (x: number) => x + 1;
            const multiplyByTwo = (x: number) => x * 2;
            const subtractThree = (x: number) => x - 3;
            const addOneMultiplyByTwoSubtractThree = pipe(
                addOne,
                multiplyByTwo,
                subtractThree,
            );
            expect(addOneMultiplyByTwoSubtractThree(5)).toBe(9);
        });
    });
}
