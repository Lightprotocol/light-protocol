/**
 * @description Computes the indices in which the asset for the utxo is in the asset pubkeys array.
 * @note Using the indices the zero knowledge proof circuit enforces that only utxos containing the
 * @note assets in the asset pubkeys array are contained in the transaction.
 * @param utxos
 * @returns
 */
// TODO: make this work for edge case of two 2 different assets plus fee asset in the same transaction
// TODO: fix edge case of an asset pubkey being 0
// TODO: !== !! and check non-null
export function getIndices3Dim(
  dimension2: number,
  dimension3: number,
  assetsCircuitArray: any[][],
  assetPubkeysCircuit: Array<string>,
): string[][][] {
  let inIndices: string[][][] = [];
  assetsCircuitArray.map((assetsCircuit) => {
    let tmpInIndices: string[][] = [];
    for (let a = 0; a < dimension2; a++) {
      let tmpInIndices1: string[] = [];

      for (let i = 0; i < dimension3; i++) {
        try {
          if (
            assetsCircuit[a].toString() ===
              assetPubkeysCircuit![i].toString() &&
            !tmpInIndices1.includes("1") &&
            assetPubkeysCircuit![i].toString() != "0"
          ) {
            tmpInIndices1.push("1");
          } else {
            tmpInIndices1.push("0");
          }
        } catch (error) {
          tmpInIndices1.push("0");
        }
      }

      tmpInIndices.push(tmpInIndices1);
    }

    inIndices.push(tmpInIndices);
  });
  return inIndices;
}
