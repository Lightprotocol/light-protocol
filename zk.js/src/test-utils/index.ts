export * from "./create-accounts";
export * from "./test-checks";
export * from "./setup-merkle-tree";
export * from "./init-lookuptable";
export * from "./constants-marketplace";
export * from "./functional-circuit";
export * from "./constants-system-verifier";
export * from "./test-rpc";
export * from "./user-test-assert-helper";
export * from "./test-transaction";
export * from "./airdrop";

export function generateRandomTestAmount(
  min: number = 0.2,
  max: number = 2,
  decimals: number,
): number {
  const randomAmount = Math.random() * (max - min) + min;
  return +randomAmount.toFixed(decimals);
}
