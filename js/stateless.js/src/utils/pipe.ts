/** pipe function */
// export const pipe =
//   (...fns: Function[]) =>
//   (x: any) =>
//     fns.reduce((v, f) => f(v), x);

export function pipe<T, R>(
  initialFunction: (arg: T) => R,
  ...functions: ((arg: R) => R)[]
): (initialValue: T) => R {
  return (initialValue: T): R =>
    functions.reduce(
      (currentValue, currentFunction) => currentFunction(currentValue),
      initialFunction(initialValue)
    );
}
