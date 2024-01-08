export * from "./createAccounts";
export * from "./testChecks";
export * from "./setUpMerkleTree";
export * from "./initLookUpTable";
export * from "./constants_market_place";
export * from "./functionalCircuit";
export * from "./constants_system_verifier";
export * from "./testRpc";
export * from "./userTestAssertHelper";
export * from "./testTransaction";
export * from "./airdrop";

export function generateRandomTestAmount(
  min: number = 0.2,
  max: number = 2,
  decimals: number,
): number {
  const randomAmount = Math.random() * (max - min) + min;
  return +randomAmount.toFixed(decimals);
}
