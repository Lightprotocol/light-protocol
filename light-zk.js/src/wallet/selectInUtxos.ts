import { PublicKey, SystemProgram } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import {
  CreateUtxoErrorCode,
  RelayerErrorCode,
  SelectInUtxosError,
  SelectInUtxosErrorCode,
  TransactionErrorCode,
  Action,
  getRecipientsAmount,
  getUtxoArrayAmount,
  Recipient,
  Utxo,
  Account,
  TOKEN_REGISTRY,
} from "../index";

// TODO: turn these into static user.class methods
export const getAmount = (u: Utxo, asset: PublicKey) => {
  return u.amounts[u.assets.indexOf(asset)];
};

export const getUtxoSum = (utxos: Utxo[], asset: PublicKey) => {
  return utxos.reduce(
    (sum, utxo) => sum.add(getAmount(utxo, asset)),
    new BN(0),
  );
};

/**
 * -------------------------------------------------------------
 * Algorithm
 *
 * assumptions:
 * - send/withdraw max 1 spl asset and sol
 *
 * general strategy:
 * - merge biggest with smallest
 * - try to keep sol amount with biggest spl utxos
 * - try to not have more than two utxos of the same spl token
 *
 * Start:
 *
 * no utxos return []
 *
 * calculate sumInSpl
 * calculate sumInSol
 *
 * check recipients contain only one spl asset
 * check amounts are plausible
 *
 *
 * get commitment hash for every utxo to have an identifier
 * sort utxos descending for spl
 *
 * if spl select biggest utxo that satisfies spl amount || or biggest spl utxo and select smallest utxo that satisfies spl amount
 * if sol check whether amount is covered already
 * else
 *    if possible select utxo with smallest spl amount that works
 */
const selectBiggestSmallest = (
  filteredUtxos: Utxo[],
  assetIndex: number,
  sumOutSpl: BN,
  threshold: number,
  mint?: PublicKey,
) => {
  var selectedUtxos: Utxo[] = [];
  var selectedUtxosAmount: BN = new BN(0);
  var selectedUtxosSolAmount: BN = new BN(0);
  // TODO: write sort that works with BN
  filteredUtxos.sort(
    (a, b) =>
      b.amounts[assetIndex].toNumber() - a.amounts[assetIndex].toNumber(),
  );

  for (var utxo = 0; utxo < filteredUtxos.length; utxo++) {
    // Init with biggest spl utxo
    if (utxo == 0) {
      selectedUtxos.push(filteredUtxos[utxo]);
      selectedUtxosAmount = selectedUtxosAmount.add(
        filteredUtxos[utxo].amounts[assetIndex],
      );
      selectedUtxosSolAmount = selectedUtxosSolAmount.add(
        filteredUtxos[utxo].amounts[0],
      );
    } else {
      // searching for the biggest in combination with the smallest combination possible
      if (
        selectedUtxosAmount
          .add(filteredUtxos[utxo].amounts[assetIndex])
          .gte(sumOutSpl)
      ) {
        selectedUtxosAmount = selectedUtxosAmount.add(
          filteredUtxos[utxo].amounts[assetIndex],
        );
        selectedUtxosSolAmount = selectedUtxosSolAmount.add(
          filteredUtxos[utxo].amounts[0],
        );

        if (selectedUtxos.length == threshold) {
          // overwrite existing utxo
          selectedUtxosAmount = selectedUtxosAmount.sub(
            selectedUtxos[1].amounts[assetIndex],
          );
          selectedUtxosSolAmount = selectedUtxosSolAmount.sub(
            selectedUtxos[1].amounts[0],
          );
          selectedUtxos[1] = filteredUtxos[utxo];
        } else {
          // add utxo
          selectedUtxos.push(filteredUtxos[utxo]);
        }
      } else {
        if (selectedUtxosAmount.lt(sumOutSpl)) {
          throw new Error(
            `Could not find a utxo combination for spl token ${mint} and amount ${sumOutSpl}`,
          );
        }
      }
    }
  }
  return { selectedUtxosSolAmount, selectedUtxos };
};

// TODO: enable users to pass in this function to use their own selection strategies
// TODO: add option how many utxos to select
export function selectInUtxos({
  utxos,
  publicMint,
  publicAmountSpl,
  publicAmountSol,
  poseidon,
  relayerFee,
  inUtxos,
  outUtxos = [],
  action,
  numberMaxInUtxos,
}: {
  publicMint?: PublicKey;
  publicAmountSpl?: BN;
  publicAmountSol?: BN;
  poseidon: any;
  relayerFee?: BN;
  utxos?: Utxo[];
  inUtxos?: Utxo[];
  outUtxos?: Utxo[];
  action: Action;
  numberMaxInUtxos: number;
}) {
  if (!publicMint && publicAmountSpl)
    throw new SelectInUtxosError(
      CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED,
      "selectInUtxos",
      "Public mint not set but public spl amount",
    );
  if (publicMint && !publicAmountSpl)
    throw new SelectInUtxosError(
      CreateUtxoErrorCode.PUBLIC_SPL_AMOUNT_UNDEFINED,
      "selectInUtxos",
      "Public spl amount not set but public mint",
    );
  if (action === Action.UNSHIELD && !publicAmountSpl && !publicAmountSol)
    throw new SelectInUtxosError(
      CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
      "selectInUtxos",
      "No public amounts defined",
    );
  if (action === Action.UNSHIELD && !relayerFee)
    throw new SelectInUtxosError(
      RelayerErrorCode.RELAYER_FEE_UNDEFINED,
      "selectInUtxos",
      "Relayer fee undefined",
    );
  if (action === Action.TRANSFER && !relayerFee)
    throw new SelectInUtxosError(
      RelayerErrorCode.RELAYER_FEE_UNDEFINED,
      "selectInUtxos",
      "Relayer fee undefined",
    );

  if ((!utxos || utxos.length === 0) && action === Action.SHIELD) return [];
  else if (!utxos || utxos.length === 0)
    throw new SelectInUtxosError(
      TransactionErrorCode.NO_UTXOS_PROVIDED,
      "selectInUtxos",
      `No utxos defined for ${action}`,
    );

  if (action === Action.SHIELD && relayerFee)
    throw new SelectInUtxosError(
      CreateUtxoErrorCode.RELAYER_FEE_DEFINED,
      "selectInUtxos",
      "Relayer fee should not be defined with shield",
    );
  // TODO: evaluate whether this is too much of a footgun
  if (action === Action.SHIELD) {
    publicAmountSol = new BN(0);
    publicAmountSpl = new BN(0);
  }

  // TODO: add check that utxo holds sufficient balance

  if (outUtxos.length > 1)
    throw new SelectInUtxosError(
      CreateUtxoErrorCode.INVALID_NUMER_OF_RECIPIENTS,
      "selectInUtxos",
      `outUtxos.length ${outUtxos.length}`,
    );

  // check publicMint and recipients mints are all the same
  let mint = publicMint;
  for (var utxo of outUtxos) {
    if (!mint && utxo.amounts[1]?.gt(new BN(0))) mint = utxo.assets[1];
    if (mint && mint.toBase58() !== utxo.assets[1].toBase58())
      throw new SelectInUtxosError(
        SelectInUtxosErrorCode.INVALID_NUMER_OF_MINTS,
        "selectInUtxos",
        `Too many different mints in recipients outUtxos ${utxo}`,
      );
  }

  // if mint is provided filter for only utxos that contain the mint
  let filteredUtxos: Utxo[] = [];
  var sumInSpl = new BN(0);
  var sumInSol = getUtxoArrayAmount(SystemProgram.programId, utxos);
  var sumOutSpl = publicAmountSpl ? publicAmountSpl : new BN(0);
  var sumOutSol = getUtxoArrayAmount(SystemProgram.programId, outUtxos);
  if (relayerFee) sumOutSol = sumOutSol.add(new BN(relayerFee));
  if (publicAmountSol) sumOutSol = sumOutSol.add(publicAmountSol);

  if (mint) {
    filteredUtxos = utxos.filter((utxo) =>
      utxo.assets.find((asset) => asset.toBase58() === mint?.toBase58()),
    );
    sumInSpl = getUtxoArrayAmount(mint, filteredUtxos);
    sumInSol = getUtxoArrayAmount(SystemProgram.programId, filteredUtxos);
    sumOutSpl = getUtxoArrayAmount(mint, outUtxos);
  } else {
    filteredUtxos = utxos;
  }

  // TODO: make work with input utxo
  // if (utxos.length == 1) return [...utxos];
  var selectedUtxosR: Utxo[] = inUtxos ? [...inUtxos] : [];
  if (numberMaxInUtxos - selectedUtxosR.length < 0)
    throw new SelectInUtxosError(
      SelectInUtxosErrorCode.INVALID_NUMBER_OF_IN_UTXOS,
      "selectInUtxos",
    );
  if (mint != TOKEN_REGISTRY.get("SOL")?.mint) {
    var { selectedUtxosSolAmount, selectedUtxos } = selectBiggestSmallest(
      filteredUtxos,
      1,
      sumOutSpl,
      numberMaxInUtxos - selectedUtxosR.length,
      mint,
    );

    if (selectedUtxos.length === 0)
      throw new SelectInUtxosError(
        SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
        "selectInUtxos",
        `Failed to find any utxo of this token${utxos}`,
      );
    selectedUtxosR = selectedUtxos;

    // if sol amount not satisfied
    if (sumOutSol.gt(selectedUtxosSolAmount)) {
      // filter for utxos which could satisfy
      filteredUtxos = utxos.filter((utxo) =>
        utxo.amounts[0].gte(sumOutSol.sub(selectedUtxosSolAmount)),
      );

      // if one spl utxo is enough try to find one sol utxo which can make up the difference in all utxos with only sol
      if (selectedUtxosR[0].amounts[1].gte(sumOutSpl)) {
        // exclude the utxo which is already selected and utxos which hold other assets than only sol
        let reFilteredUtxos = utxos.filter(
          (utxo) =>
            utxo.getCommitment(poseidon) !=
              selectedUtxosR[0].getCommitment(poseidon) &&
            utxo.assets[1].toBase58() === SystemProgram.programId.toBase58(),
        );

        // search for suitable sol utxo in remaining utxos
        var { selectedUtxosSolAmount, selectedUtxos: selectedUtxo1 } =
          selectBiggestSmallest(
            reFilteredUtxos,
            1,
            sumOutSol.sub(selectedUtxosR[0].amounts[0]),
            1,
          );

        // if a sol utxo was found replace small spl utxo
        if (selectedUtxo1.length === 0)
          throw new SelectInUtxosError(
            SelectInUtxosErrorCode.FAILED_TO_SELECT_SOL_UTXO,
            "selectInUtxos",
            `Failed to select a sol utxo sumOutSol ${sumOutSol}, sumInSol ${sumInSol}`,
          );
        if (selectedUtxosR.length == numberMaxInUtxos) {
          // overwrite existing utxo
          selectedUtxosR[1] = selectedUtxo1[0];
        } else {
          // add utxo
          selectedUtxosR.push(selectedUtxo1[0]);
        }
      }
      // take utxo with smallest spl amount of utxos which satisfy
      else if (filteredUtxos.length === 0) {
        throw new SelectInUtxosError(
          SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
          "selectInUtxos",
          `Could not find a utxo combination for spl token ${mint} and amount ${sumOutSpl} and sol amount ${sumOutSol}`,
        );
      } else {
        // sort ascending and take smallest index
        filteredUtxos.sort((a, b) => a.amounts[1].sub(b.amounts[1]).toNumber());
        if (selectedUtxosR.length == numberMaxInUtxos) {
          // overwrite existing utxo
          selectedUtxosR[1] = filteredUtxos[0];
        } else {
          // add utxo
          selectedUtxosR.push(filteredUtxos[0]);
        }
      }
    }
  } else {
    // case no spl amount only select sol
    var { selectedUtxos } = selectBiggestSmallest(
      filteredUtxos,
      0,
      sumOutSol,
      numberMaxInUtxos - selectedUtxosR.length,
      mint,
    );
    selectedUtxosR = selectedUtxos;
  }

  if (mint && !getUtxoArrayAmount(mint, selectedUtxosR).gte(sumOutSpl))
    throw new SelectInUtxosError(
      SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
      "selectInUtxos",
      `Failed to get spl amount requested ${sumOutSpl} possible ${getUtxoArrayAmount(
        mint,
        selectedUtxosR,
      )}`,
    );
  if (
    !getUtxoArrayAmount(SystemProgram.programId, selectedUtxosR).gte(sumOutSol)
  )
    throw new SelectInUtxosError(
      SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
      "selectInUtxos",
      `Failed to get sol amount requested ${sumOutSol} possible ${getUtxoArrayAmount(
        SystemProgram.programId,
        selectedUtxosR,
      )}`,
    );

  if (selectedUtxosR.length > 0) return selectedUtxosR;
  else if (action === Action.SHIELD) return [];
  else
    throw new SelectInUtxosError(
      SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
      "selectInUtxos",
      "selectInUtxos failed to select utxos",
    );
}
