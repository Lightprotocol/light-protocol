import { PublicKey } from "@solana/web3.js";
import {
  UTXO_MERGE_THRESHOLD,
  UTXO_MERGE_MAXIMUM,
  FEE_ASSET,
  UTXO_FEE_ASSET_MINIMUM,
} from "../constants";
import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";

// TODO: turn these into static user.class methods
const getAmount = (u: Utxo, asset: PublicKey) => {
  return u.amounts[u.assets.indexOf(asset)];
};

const getFeeSum = (utxos: Utxo[]) => {
  return utxos.reduce(
    (sum, utxo) => sum + getAmount(utxo, FEE_ASSET).toNumber(),
    0,
  );
};

export function selectInUtxos({
  mint,
  amount,
  extraSolAmount,
  utxos,
}: {
  mint: PublicKey;
  amount: number;
  extraSolAmount: number;
  utxos: Utxo[];
}) {
  // TODO: verify that this is correct w -
  if (utxos === undefined) return [];
  if (utxos.length >= UTXO_MERGE_THRESHOLD)
    return [...utxos.slice(0, UTXO_MERGE_MAXIMUM)];
  if (utxos.length == 1) return [...utxos]; // TODO: check if this still works for spl...

  var options: Utxo[] = [];

  utxos = utxos.filter((utxo) => utxo.assets.includes(mint));
  var extraSolUtxos;
  if (mint !== FEE_ASSET) {
    extraSolUtxos = utxos
      .filter((utxo) => {
        let i = utxo.amounts.findIndex((amount) => amount === new anchor.BN(0));
        // The other asset must be 0 and SOL must be >0
        return (
          utxo.assets.includes(FEE_ASSET) &&
          i !== -1 &&
          utxo.amounts[utxo.assets.indexOf(FEE_ASSET)] > new anchor.BN(0)
        );
      })
      .sort(
        (a, b) =>
          getAmount(a, FEE_ASSET).toNumber() -
          getAmount(b, FEE_ASSET).toNumber(),
      );
  } else console.log("mint is FEE_ASSET");

  /**
   * for shields and transfers we'll always have spare utxos,
   * hence no reason to find perfect matches
   * */
  if (amount > 0) {
    // perfect match (2-in, 0-out)
    for (let i = 0; i < utxos.length; i++) {
      for (let j = 0; j < utxos.length; j++) {
        if (i == j || getFeeSum([utxos[i], utxos[j]]) < UTXO_FEE_ASSET_MINIMUM)
          continue;
        else if (
          getAmount(utxos[i], mint).add(getAmount(utxos[j], mint)) ==
          new anchor.BN(amount)
        ) {
          options.push(utxos[i], utxos[j]);
          return options;
        }
      }
    }

    // perfect match (1-in, 0-out)
    if (options.length < 1) {
      let match = utxos.filter(
        (utxo) => getAmount(utxo, mint) == new anchor.BN(amount),
      );
      if (match.length > 0) {
        const sufficientFeeAsset = match.filter(
          (utxo) => getFeeSum([utxo]) >= UTXO_FEE_ASSET_MINIMUM,
        );
        if (sufficientFeeAsset.length > 0) {
          options.push(sufficientFeeAsset[0]);
          return options;
        } else if (extraSolUtxos && extraSolUtxos.length > 0) {
          options.push(match[0]);
          /** handler 1: 2 in - 1 out here, with a feeutxo merged into place */
          /** TODO:  add as fallback: use another MINT utxo */
          // Find the smallest sol utxo that can cover the fee
          for (let i = 0; i < extraSolUtxos.length; i++) {
            if (
              getFeeSum([match[0], extraSolUtxos[i]]) >= UTXO_FEE_ASSET_MINIMUM
            ) {
              options.push(extraSolUtxos[i]);
              break;
            }
          }
          return options;
        }
      }
    }
  }

  // 2 above amount - find the pair of the UTXO with the largest amount and the UTXO of the smallest amount, where its sum is greater than amount.
  if (options.length < 1) {
    for (let i = 0; i < utxos.length; i++) {
      for (let j = utxos.length - 1; j >= 0; j--) {
        if (i == j || getFeeSum([utxos[i], utxos[j]]) < UTXO_FEE_ASSET_MINIMUM)
          continue;
        else if (
          getAmount(utxos[i], mint).add(getAmount(utxos[j], mint)) >
          new anchor.BN(amount)
        ) {
          options.push(utxos[i], utxos[j]);
          return options;
        }
      }
    }
  }

  // if 2-in is not sufficient to cover the transaction amount, use 10-in -> merge everything
  // cases where utxos.length > UTXO_MERGE_MAXIMUM are handled above already
  if (
    options.length < 1 &&
    utxos.reduce((a, b) => a + getAmount(b, mint).toNumber(), 0) >= amount
  ) {
    if (
      getFeeSum(utxos.slice(0, UTXO_MERGE_MAXIMUM)) >= UTXO_FEE_ASSET_MINIMUM
    ) {
      options = [...utxos.slice(0, UTXO_MERGE_MAXIMUM)];
    } else if (extraSolUtxos && extraSolUtxos.length > 0) {
      // get a utxo set of (...utxos.slice(0, UTXO_MERGE_MAXIMUM-1)) that can cover the amount.
      // skip last one to the left (smallest utxo!)
      for (let i = utxos.length - 1; i > 0; i--) {
        let sum = 0;
        let utxoSet = [];
        for (let j = i; j > 0; j--) {
          if (sum >= amount) break;
          sum += getAmount(utxos[j], mint).toNumber();
          utxoSet.push(utxos[j]);
        }
        if (sum >= amount && getFeeSum(utxoSet) >= UTXO_FEE_ASSET_MINIMUM) {
          options = utxoSet;
          break;
        }
      }
      // find the smallest sol utxo that can cover the fee
      if (options && options.length > 0)
        for (let i = 0; i < extraSolUtxos.length; i++) {
          if (
            getFeeSum([
              ...utxos.slice(0, UTXO_MERGE_MAXIMUM),
              extraSolUtxos[i],
            ]) >= UTXO_FEE_ASSET_MINIMUM
          ) {
            options = [...options, extraSolUtxos[i]];
            break;
          }
        }
      // TODO: add as fallback: use another MINT/third spl utxo
    }
  }
  return options;
}
