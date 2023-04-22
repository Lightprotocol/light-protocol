import { PublicKey, SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  CreateUtxoError,
  CreateUtxoErrorCode,
  TransactionErrorCode,
  TransactionParametersErrorCode,
  Action,
  Account,
  Utxo,
  TransactionParameters,
  AppUtxoConfig,
} from "../index";

type Asset = { sumIn: BN; sumOut: BN; asset: PublicKey };

export type Recipient = {
  account: Account;
  solAmount: BN;
  splAmount: BN;
  mint: PublicKey;
  appUtxo?: AppUtxoConfig;
};
// mint: PublicKey, expectedAmount: BN,
export const getUtxoArrayAmount = (mint: PublicKey, inUtxos: Utxo[]) => {
  let inAmount = new BN(0);
  inUtxos.forEach((inUtxo) => {
    inUtxo.assets.forEach((asset, i) => {
      if (asset.toBase58() === mint.toBase58()) {
        inAmount = inAmount.add(inUtxo.amounts[i]);
      }
    });
  });
  return inAmount;
};

export const getRecipientsAmount = (
  mint: PublicKey,
  recipients: Recipient[],
) => {
  if (mint.toBase58() === SystemProgram.programId.toBase58()) {
    return recipients.reduce(
      (sum, recipient) => sum.add(recipient.solAmount),
      new BN(0),
    );
  } else {
    return recipients.reduce(
      (sum, recipient) =>
        recipient.mint.toBase58() === mint.toBase58()
          ? sum.add(recipient.splAmount)
          : sum.add(new BN(0)),
      new BN(0),
    );
  }
};
// --------------------------------------------------------------------------
// Algorithm:
// check nr recipients is leq to nrOuts of verifier
// check that publicMint and recipientMints exist in inputUtxos
// checks sum inAmounts for every asset are less or equal to sum OutAmounts

// unshield
// publicSol -sumSolAmount
// publicSpl -sumSplAmount
// publicMint

// transfer
// check no publics

// shield
// sumInSol +sumSolAmount
// sumInSpl +sumSplAmount
// publicMint

// create via recipients requested utxos and subtract amounts from sums
// beforeEach utxo check that no amount is negative

// create change utxos with remaining spl balances and sol balance
// --------------------------------------------------------------------------

export function createMissingOutUtxos({
  poseidon,
  inUtxos,
  outUtxos = [],
  publicMint,
  publicAmountSpl,
  publicAmountSol,
  relayerFee,
  changeUtxoAccount,
  action,
  appUtxo,
  numberMaxOutUtxos,
}: {
  inUtxos?: Utxo[];
  publicMint?: PublicKey;
  publicAmountSpl?: BN;
  publicAmountSol?: BN;
  relayerFee?: BN;
  poseidon: any;
  changeUtxoAccount: Account;
  outUtxos?: Utxo[];
  action: Action;
  appUtxo?: AppUtxoConfig;
  numberMaxOutUtxos: number;
}) {
  if (!poseidon)
    throw new CreateUtxoError(
      TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
      "createMissingOutUtxos",
      "Poseidon not initialized",
    );

  if (relayerFee) {
    publicAmountSol = publicAmountSol
      ? publicAmountSol.add(relayerFee)
      : relayerFee;
  }

  const { assetPubkeysCircuit, assetPubkeys } =
    !inUtxos && action === Action.SHIELD
      ? {
          assetPubkeys: [
            SystemProgram.programId,
            publicMint ? publicMint : SystemProgram.programId,
          ],
          assetPubkeysCircuit: [],
        }
      : TransactionParameters.getAssetPubkeys(inUtxos);
  if (!assetPubkeys)
    throw new CreateUtxoError(
      TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED,
      "constructor",
    );

  // TODO: enable perfect manual amounts of amounts to recipients
  // check nr outUtxos is leq to nrOuts of verifier
  if (outUtxos.length > 1) {
    throw new CreateUtxoError(
      CreateUtxoErrorCode.INVALID_NUMER_OF_RECIPIENTS,
      "createMissingOutUtxos",
      `Number of recipients greater than allowed: ${
        outUtxos.length
      } allowed ${1}`,
    );
  }

  // recipients.map((recipient) => {
  //   if (
  //     !assetPubkeys.find((x) => x.toBase58() === recipient.mint?.toBase58())
  //   ) {
  //     throw new CreateUtxoError(
  //       CreateUtxoErrorCode.INVALID_RECIPIENT_MINT,
  //       "createMissingOutUtxos",
  //       `Mint ${recipient.mint} does not exist in input utxos mints ${assetPubkeys}`,
  //     );
  //   }
  // });
  outUtxos.map((outUtxo) => {
    if (
      !assetPubkeys.find((x) => x.toBase58() === outUtxo.assets[1]?.toBase58())
    ) {
      throw new CreateUtxoError(
        CreateUtxoErrorCode.INVALID_RECIPIENT_MINT,
        "createMissingOutUtxos",
        `Mint ${outUtxo.assets[1]} does not exist in input utxos mints ${assetPubkeys}`,
      );
    }
  });
  // add public mint if it does not exist in inUtxos
  if (
    publicMint &&
    !assetPubkeys.find((x) => x.toBase58() === publicMint.toBase58())
  ) {
    assetPubkeys.push(publicMint);
  }

  // checks sum inAmounts for every asset are less or equal to sum OutAmounts
  // for (var i in assetPubkeys) {
  //   const sumIn = inUtxos
  //     ? getUtxoArrayAmount(assetPubkeys[i], inUtxos)
  //     : new BN(0);
  //   const sumOut = getRecipientsAmount(assetPubkeys[i], recipients);

  //   assets.push({
  //     asset: assetPubkeys[i],
  //     sumIn,
  //     sumOut,
  //   });

  //   if (!sumIn.gte(sumOut)) {
  //     throw new CreateUtxoError(
  //       CreateUtxoErrorCode.RECIPIENTS_SUM_AMOUNT_MISSMATCH,
  //       "createMissingOutUtxos",
  //       `for asset ${assetPubkeys[
  //         i
  //       ].toBase58()} sumOut ${sumOut} greather than sumIN ${sumIn}`,
  //     );
  //   }
  // }
  let assets: Asset[] = validateUtxoAmounts({
    assetPubkeys,
    inUtxos,
    outUtxos,
  });
  let publicSolAssetIndex = assets.findIndex(
    (x) => x.asset.toBase58() === SystemProgram.programId.toBase58(),
  );

  // remove duplicates
  const key = "asset";
  assets = [...new Map(assets.map((item) => [item[key], item])).values()];

  // subtract public amounts from sumIns
  if (action === Action.UNSHIELD) {
    if (!publicAmountSol && !publicAmountSpl)
      throw new CreateUtxoError(
        CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        "createMissingOutUtxos",
        "publicAmountSol not initialized for unshield",
      );
    if (!publicAmountSpl) publicAmountSpl = new BN(0);
    if (!publicAmountSol)
      throw new CreateUtxoError(
        CreateUtxoErrorCode.PUBLIC_SOL_AMOUNT_UNDEFINED,
        "constructor",
      );

    if (publicAmountSpl && !publicMint)
      throw new CreateUtxoError(
        CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED,
        "createMissingOutUtxos",
        "publicMint not initialized for unshield",
      );

    let publicSplAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === publicMint?.toBase58(),
    );

    assets[publicSplAssetIndex].sumIn =
      assets[publicSplAssetIndex].sumIn.sub(publicAmountSpl);
    assets[publicSolAssetIndex].sumIn =
      assets[publicSolAssetIndex].sumIn.sub(publicAmountSol);
    // add public amounts to sumIns
  } else if (action === Action.SHIELD) {
    if (relayerFee)
      throw new CreateUtxoError(
        CreateUtxoErrorCode.RELAYER_FEE_DEFINED,
        "createMissingOutUtxos",
        "Shield and relayer fee defined",
      );
    if (!publicAmountSpl) publicAmountSpl = new BN(0);
    if (!publicAmountSol) publicAmountSol = new BN(0);
    let publicSplAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === publicMint?.toBase58(),
    );
    let publicSolAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === SystemProgram.programId.toBase58(),
    );
    assets[publicSplAssetIndex].sumIn =
      assets[publicSplAssetIndex].sumIn.add(publicAmountSpl);
    assets[publicSolAssetIndex].sumIn =
      assets[publicSolAssetIndex].sumIn.add(publicAmountSol);
  } else if (action === Action.TRANSFER) {
    if (!publicAmountSol)
      throw new CreateUtxoError(
        CreateUtxoErrorCode.PUBLIC_SOL_AMOUNT_UNDEFINED,
        "constructor",
      );
    let publicSolAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === SystemProgram.programId.toBase58(),
    );

    assets[publicSolAssetIndex].sumIn =
      assets[publicSolAssetIndex].sumIn.sub(publicAmountSol);
  }

  var outputUtxos: Utxo[] = [...outUtxos];

  // create recipient output utxos, one for each defined recipient
  for (var j in outUtxos) {
    if (outUtxos[j].assets[1] && !outUtxos[j].amounts[1]) {
      throw new CreateUtxoError(
        CreateUtxoErrorCode.SPL_AMOUNT_UNDEFINED,
        "createMissingOutUtxos",
        `Mint defined while splAmount is undefinedfor recipient ${outUtxos[j]}`,
      );
    }

    let solAmount = outUtxos[j].amounts[0] ? outUtxos[j].amounts[0] : new BN(0);
    let splAmount = outUtxos[j].amounts[1] ? outUtxos[j].amounts[1] : new BN(0);
    let splMint = outUtxos[j].assets[1]
      ? outUtxos[j].assets[1]
      : SystemProgram.programId;

    let publicSplAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === splMint?.toBase58(),
    );

    assets[publicSplAssetIndex].sumIn = assets[publicSplAssetIndex].sumIn
      .sub(splAmount)
      .clone();
    assets[publicSolAssetIndex].sumIn = assets[publicSolAssetIndex].sumIn
      .sub(solAmount)
      .clone();
  }
  // create change utxo
  // Also handles case that we have more than one change utxo because we wanted to withdraw sol and used utxos with different spl tokens
  // it creates a change utxo for every asset that is non-zero then check that number of utxos is less or equal to verifier.config.outs
  let publicSplAssets = assets.filter(
    (x) =>
      x.sumIn.toString() !== "0" &&
      x.asset.toBase58() !== SystemProgram.programId.toBase58(),
  );

  const nrOutUtxos = publicSplAssets.length ? publicSplAssets.length : 1;

  for (var x = 0; x < nrOutUtxos; x++) {
    let solAmount = new BN(0);
    if (x == 0) {
      solAmount = assets[publicSolAssetIndex].sumIn;
    }
    // catch case of sol deposit with undefined spl assets
    let splAmount = publicSplAssets[x]?.sumIn
      ? publicSplAssets[x].sumIn
      : new BN(0);
    let splAsset = publicSplAssets[x]?.asset
      ? publicSplAssets[x].asset
      : SystemProgram.programId;

    let changeUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, splAsset],
      amounts: [solAmount, splAmount],
      account: changeUtxoAccount,
      appData: appUtxo?.appData,
      appDataHash: appUtxo?.appDataHash,
      includeAppData: appUtxo?.includeAppData,
      verifierAddress: appUtxo?.verifierAddress,
    });
    outputUtxos.push(changeUtxo);
  }

  if (outputUtxos.length > numberMaxOutUtxos) {
    throw new CreateUtxoError(
      CreateUtxoErrorCode.INVALID_OUTPUT_UTXO_LENGTH,
      "createMissingOutUtxos",
      `Probably too many input assets possibly in combination with an incompatible number of shielded recipients ${outputUtxos}`,
    );
  }
  return outputUtxos;
}

/**
 * @description Creates an array of UTXOs for each recipient based on their specified amounts and assets.
 *
 * @param recipients - Array of Recipient objects containing the recipient's account, SOL and SPL amounts, and mint.
 * @param poseidon - A Poseidon instance for hashing.
 *
 * @throws CreateUtxoError if a recipient has a mint defined but the SPL amount is undefined.
 * @returns An array of Utxos, one for each recipient.
 */
export function createRecipientUtxos({
  recipients,
  poseidon,
}: {
  recipients: Recipient[];
  poseidon: any;
}): Utxo[] {
  var outputUtxos: Utxo[] = [];

  // create recipient output utxos, one for each defined recipient
  for (var j in recipients) {
    if (recipients[j].mint && !recipients[j].splAmount) {
      throw new CreateUtxoError(
        CreateUtxoErrorCode.SPL_AMOUNT_UNDEFINED,
        "createMissingOutUtxos",
        `Mint defined while splAmount is undefinedfor recipient ${recipients[j]}`,
      );
    }

    let solAmount = recipients[j].solAmount
      ? recipients[j].solAmount
      : new BN(0);
    let splAmount = recipients[j].splAmount
      ? recipients[j].splAmount
      : new BN(0);
    let splMint = recipients[j].mint
      ? recipients[j].mint
      : SystemProgram.programId;

    let recipientUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, splMint],
      amounts: [solAmount, splAmount],
      account: recipients[j].account,
      appData: recipients[j].appUtxo?.appData,
      includeAppData: recipients[j].appUtxo?.includeAppData,
      appDataHash: recipients[j].appUtxo?.appDataHash,
      verifierAddress: recipients[j].appUtxo?.verifierAddress,
    });

    outputUtxos.push(recipientUtxo);
  }
  return outputUtxos;
}

/**
 * @description Validates if the sum of input UTXOs for each asset is less than or equal to the sum of output UTXOs.
 *
 * @param assetPubkeys - Array of PublicKeys representing the asset public keys to be checked.
 * @param inUtxos - Array of input UTXOs containing the asset amounts being spent.
 * @param outUtxos - Array of output UTXOs containing the asset amounts being received.
 *
 * @throws Error if the sum of input UTXOs for an asset is less than the sum of output UTXOs.
 */
export function validateUtxoAmounts({
  assetPubkeys,
  inUtxos,
  outUtxos,
}: {
  assetPubkeys: PublicKey[];
  inUtxos?: Utxo[];
  outUtxos: Utxo[];
}): Asset[] {
  let assets: Asset[] = [];
  for (const assetPubkey of assetPubkeys) {
    const sumIn = inUtxos
      ? getUtxoArrayAmount(assetPubkey, inUtxos)
      : new BN(0);
    const sumOut = getUtxoArrayAmount(assetPubkey, outUtxos);
    assets.push({
      asset: assetPubkey,
      sumIn,
      sumOut,
    });
    if (!sumIn.gte(sumOut)) {
      throw new CreateUtxoError(
        CreateUtxoErrorCode.RECIPIENTS_SUM_AMOUNT_MISSMATCH,
        "validateUtxoAmounts",
        `for asset ${assetPubkey.toBase58()} sumOut ${sumOut} greather than sumIN ${sumIn}`,
      );
    }
  }
  return assets;
}
