import { PublicKey, SystemProgram } from "@solana/web3.js";
import { Utxo } from "../utxo";
import { Account } from "../account";
import { Action, TransactionParameters } from "../transaction";
import { BN } from "@coral-xyz/anchor";
import {
  CreateUtxoError,
  CreateUtxoErrorCode,
  TransactionErrorCode,
  TransactionParametersErrorCode,
} from "../errors";

type Asset = { sumIn: BN; sumOut: BN; asset: PublicKey };

export type Recipient = {
  account: Account;
  solAmount: BN;
  splAmount: BN;
  mint: PublicKey;
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

// TODO: handle passed in outputUtxo and create change utxo for that
export function createOutUtxos({
  poseidon,
  inUtxos,
  publicMint,
  publicAmountSpl,
  publicAmountSol,
  relayerFee,
  changeUtxoAccount,
  recipients = [],
  action,
}: {
  inUtxos?: Utxo[];
  publicMint?: PublicKey;
  publicAmountSpl?: BN;
  publicAmountSol?: BN;
  relayerFee?: BN;
  poseidon: any;
  changeUtxoAccount: Account;
  recipients?: Recipient[];
  action: Action;
}) {
  if (!poseidon)
    throw new CreateUtxoError(
      TransactionParametersErrorCode.NO_POSEIDON_HASHER_PROVIDED,
      "createOutUtxos",
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
  let assets: Asset[] = [];

  // TODO: make flexible with different verifiers
  // TODO: enable perfect manual amounts of amounts to recipients
  // check nr recipients is leq to nrOuts of verifier
  if (recipients.length > 1) {
    throw new CreateUtxoError(
      CreateUtxoErrorCode.INVALID_NUMER_OF_RECIPIENTS,
      "createOutUtxos",
      `Number of recipients greater than allowed: ${
        recipients.length
      } allowed ${1}`,
    );
  }

  recipients.map((recipient) => {
    if (
      !assetPubkeys.find((x) => x.toBase58() === recipient.mint?.toBase58())
    ) {
      throw new CreateUtxoError(
        CreateUtxoErrorCode.INVALID_RECIPIENT_MINT,
        "createOutUtxos",
        `Mint ${recipient.mint} does not exist in input utxos mints ${assetPubkeys}`,
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
  for (var i in assetPubkeys) {
    const sumIn = inUtxos
      ? getUtxoArrayAmount(assetPubkeys[i], inUtxos)
      : new BN(0);
    const sumOut = getRecipientsAmount(assetPubkeys[i], recipients);

    assets.push({
      asset: assetPubkeys[i],
      sumIn,
      sumOut,
    });

    if (!sumIn.gte(sumOut)) {
      throw new CreateUtxoError(
        CreateUtxoErrorCode.RECIPIENTS_SUM_AMOUNT_MISSMATCH,
        "createOutUtxos",
        `for asset ${assetPubkeys[
          i
        ].toBase58()} sumOut ${sumOut} greather than sumIN ${sumIn}`,
      );
    }
  }
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
        "createOutUtxos",
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
        "createOutUtxos",
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
        "createOutUtxos",
        "Shield and relayer fee defined",
      );
    if (!publicSolAmount && !publicSplAmount)
      throw new CreateUtxoError(
        CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        "createOutUtxos",
        "publicSolAmount not initialized for unshield",
      );
    if (!publicSplAmount) publicSplAmount = new BN(0);
    if (!publicSolAmount) publicSolAmount = new BN(0);
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

  var outputUtxos: Utxo[] = [];

  // create recipient output utxos, one for each defined recipient
  for (var j in recipients) {
    if (recipients[j].mint && !recipients[j].splAmount) {
      throw new CreateUtxoError(
        CreateUtxoErrorCode.SPL_AMOUNT_UNDEFINED,
        "createOutUtxos",
        `Mint defined while splAmount is undefinedfor recipient ${recipients[j]}`,
      );
    }
    // throws in reduce already
    // TODO: throw better error than in reduce
    // if(!recipients[j].account) {

    //   throw new CreateUtxoError(CreateUtxoErrorCode.SPL_AMOUNT_UNDEFINED,"createOutUtxos",`Recipients account is undefined ${recipients[j]}`);
    // }
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
    });
    outputUtxos.push(recipientUtxo);
    let publicSplAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === publicMint?.toBase58(),
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
    });
    outputUtxos.push(changeUtxo);
  }
  // TODO: adapt to verifier
  if (outputUtxos.length > 2) {
    throw new CreateUtxoError(
      CreateUtxoErrorCode.INVALID_OUTPUT_UTXO_LENGTH,
      "createOutUtxos",
      `Probably too many input assets possibly in combination with an incompatible number of shielded recipients ${outputUtxos}`,
    );
  }

  return outputUtxos;
}
