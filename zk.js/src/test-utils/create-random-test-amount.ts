export function generateRandomTestAmount(
  min: number = 0.2,
  max: number = 2,
  decimals: number,
): number {
  const randomAmount = Math.random() * (max - min) + min;
  return +randomAmount.toFixed(decimals);
}
