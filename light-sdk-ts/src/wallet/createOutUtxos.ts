import { PublicKey, SystemProgram } from "@solana/web3.js";
import { FEE_ASSET } from "../constants";
import { Relayer } from "../relayer";
import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Account } from "../account";
import { Action, TransactionParameters } from "../transaction";
import { BN } from "@coral-xyz/anchor";
import {
  CreateUtxoError,
  CreateUtxoErrorCode,
  TransactionParametersErrorCode,
  TransactioParametersError,
} from "../errors";

type Asset = { sumIn: BN; sumOut: BN; asset: PublicKey };

export type Recipient = {
  account: Account;
  solAmount: BN;
  splAmount: BN;
  mint: PublicKey;
};
// mint: PublicKey, expectedAmount: BN,
const getUtxoArrayAmount = (mint: PublicKey, inUtxos: Utxo[]) => {
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

const getRecipientsAmount = (mint: PublicKey, recipients: Recipient[]) => {
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

// create via recipients requested utxos and subtract amounts from sums
// beforeEach utxo check that no amount is negative

// create change utxos with remaining spl balances and sol balance
// --------------------------------------------------------------------------

// TODO: handle passed in outputUtxo and create change utxo for that
export function createOutUtxos({
  poseidon,
  inUtxos,
  publicMint,
  publicSplAmount,
  publicSolAmount,
  relayerFee,
  changeUtxoAccount,
  recipients = [],
  action,
}: {
  inUtxos: Utxo[];
  publicMint?: PublicKey;
  publicSplAmount?: BN;
  publicSolAmount?: BN;
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
    publicSolAmount = publicSolAmount
      ? publicSolAmount.add(relayerFee)
      : relayerFee;
  }

  const { assetPubkeysCircuit, assetPubkeys } =
    TransactionParameters.getAssetPubkeys(inUtxos);

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
    if (assetPubkeys.indexOf(recipient.mint) === -1) {
      throw new CreateUtxoError(
        CreateUtxoErrorCode.INVALID_RECIPIENT_MINT,
        "createOutUtxos",
        `Mint ${recipient.mint} does not exist in input utxos mints ${assetPubkeys}`,
      );
    }
  });

  // checks sum inAmounts for every asset are less or equal to sum OutAmounts
  for (var i in assetPubkeys) {
    const sumIn = getUtxoArrayAmount(assetPubkeys[i], inUtxos);
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

  // subtract public amounts from sumIns
  if (action === Action.UNSHIELD) {
    if (!publicSolAmount && !publicSplAmount)
      throw new CreateUtxoError(
        CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        "createOutUtxos",
        "publicSolAmount not initialized for unshield",
      );
    if (!publicSplAmount) publicSplAmount = new BN(0);
    if (!publicSolAmount) publicSolAmount = new BN(0);
    if (publicSplAmount && !publicMint)
      throw new CreateUtxoError(
        CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED,
        "createOutUtxos",
        "publicMint not initialized for unshield",
      );

    let publicSplAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === publicMint?.toBase58(),
    );
    let publicSolAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === SystemProgram.programId.toBase58(),
    );
    assets[publicSplAssetIndex].sumIn =
      assets[publicSplAssetIndex].sumIn.sub(publicSplAmount);
    assets[publicSolAssetIndex].sumIn =
      assets[publicSolAssetIndex].sumIn.sub(publicSolAmount);
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
    let publicSolAssetIndex = assets.findIndex(
      (x) => x.asset.toBase58() === SystemProgram.programId.toBase58(),
    );
    assets[publicSplAssetIndex].sumIn =
      assets[publicSplAssetIndex].sumIn.sub(splAmount);
    assets[publicSolAssetIndex].sumIn =
      assets[publicSolAssetIndex].sumIn.sub(solAmount);
  }
  // create change utxo
  // Also handles case that we have more than one change utxo because we wanted to withdraw sol and used utxos with different spl tokens
  // it creates a change utxo for every asset that is non-zero then check that number of utxos is less or equal to verifier.config.outs
  let publicSplAssets = assets.filter(
    (x) =>
      x.sumIn.toString() !== "0" &&
      x.asset.toBase58() !== SystemProgram.programId.toBase58(),
  );
  let publicSolAssetIndex = assets.findIndex(
    (x) => x.asset.toBase58() === SystemProgram.programId.toBase58(),
  );
  for (var x = 0; x < publicSplAssets.length; x++) {
    let solAmount = new BN(0);
    if (x == 0) {
      solAmount = assets[publicSolAssetIndex].sumIn;
    }
    let changeUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, assets[1].asset],
      amounts: [solAmount, publicSplAssets[x].sumIn],
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

  // assets[publicSolAssetIndex].sumIn = assets[publicSolAssetIndex].sumIn.sub(assets[publicSolAssetIndex].sumIn);
  // assets[publicSplAssetIndex].sumIn = assets[publicSplAssetIndex].sumIn.sub(assets[publicSplAssetIndex].sumIn);
  // console.log("change utxo", changeUtxo);

  // for (var i in assets) {
  //   if (!assets[i].sumIn.eq(new BN(0))) {
  //     throw new Error(
  //       `asset ${assets[i].asset.toBase58()} not completely allocated ${
  //         assets[i].sumIn
  //       }`,
  //     );
  //   }
  // }
  return outputUtxos;

  /*
  if (action === Action.UNSHIELD) {
    let inAmount = getAmount();
    
    if (inAmount < Math.abs(publicSplAmount)) {
      throw new Error(
        `Insufficient funds for unshield/transfer. In publicSplAmount: ${inAmount}, out publicSplAmount: ${publicSplAmount}`,
      );
    }
  }

  type Asset = { amount: number; asset: PublicKey };
  let assets: Asset[] = [];
  assets.push({
    asset: SystemProgram.programId,
    amount: 0,
  });
  /// For shields: add amount to asset for out
  // TODO: for spl-might want to consider merging 2-1 as outs .
  let assetIndex = assets.findIndex(
    (a) => a.asset.toBase58() === mint.toBase58(),
  );
  if (assetIndex === -1) {
    assets.push({ asset: mint, amount: action !== Action.TRANSFER ? publicSplAmount : 0 });
  } else {
    assets[assetIndex].amount += action !== Action.TRANSFER ? publicSplAmount : 0;
  }

  // add in-amounts to assets
  inUtxos.forEach((inUtxo) => {
    inUtxo.assets.forEach((asset, i) => {
      let assetAmount = inUtxo.amounts[i].toNumber();
      let assetIndex = assets.findIndex(
        (a) => a.asset.toBase58() === asset.toBase58(),
      );

      if (assetIndex === -1) {
        assets.push({ asset, amount: assetAmount });
      } else {
        assets[assetIndex].amount += assetAmount;
      }
    });
  });
  let feeAsset = assets.find(
    (a) => a.asset.toBase58() === FEE_ASSET.toBase58(),
  );
  if (!feeAsset) throw new Error("Fee asset not found in assets");

  if (assets.length === 1 || assetIndex === 0) {
    // just fee asset as oututxo

    if (action === Action.TRANSFER) {
      let feeAssetSendUtxo = new Utxo({
        poseidon,
        assets: [assets[0].asset],
        amounts: [new anchor.BN(publicSolAmount)],
        account: new Account({
          poseidon: poseidon,
          publicKey: recipientAccount.pubkey,
          encryptionPublicKey: recipientAccount.encryptionKeypair.publicKey,
        }),
      });

      let feeAssetChangeUtxo = new Utxo({
        poseidon,
        assets: [
          assets[0].asset,
          assets[1] ? assets[1].asset : assets[0].asset,
        ],
        amounts: [
          new anchor.BN(assets[0].amount)
            .sub(new anchor.BN(publicSolAmount))
            .sub(relayer?.relayerFee || new anchor.BN(0)), // sub from change
          assets[1] ? new anchor.BN(assets[1].amount) : new anchor.BN(0),
        ], // rem transfer positive
        account: senderAccount,
      });

      return [feeAssetSendUtxo, feeAssetChangeUtxo];
    } else {
      let feeAssetChangeUtxo = new Utxo({
        poseidon,
        assets: [
          assets[0].asset,
          assets[1] ? assets[1].asset : assets[0].asset,
        ],
        amounts: [
          action !== Action.UNSHIELD
            ? new anchor.BN(publicSolAmount + assets[0].amount)
            : new anchor.BN(assets[0].amount),
          assets[1] ? new anchor.BN(assets[1].amount) : new anchor.BN(0),
        ],
        account: recipientAccount
          ? recipientAccount
          : senderAccount,
      });

      return [feeAssetChangeUtxo];
    }
  } else {
    if (action === Action.TRANSFER) {
      let sendAmountFeeAsset = new anchor.BN(1e5);

      let sendUtxo = new Utxo({
        poseidon,
        assets: [assets[0].asset, assets[1].asset],
        amounts: [sendAmountFeeAsset, new anchor.BN(publicSplAmount)],
        account: new Account({
          poseidon: poseidon,
          publicKey: recipientAccount.pubkey,
          encryptionPublicKey: recipientAccount.encryptionKeypair.publicKey,
        }),
      });
      let changeUtxo = new Utxo({
        poseidon,
        assets: [assets[0].asset, assets[1].asset],
        amounts: [
          new anchor.BN(assets[0].amount)
            .sub(sendAmountFeeAsset)
            .sub(relayer?.relayerFee || new anchor.BN(0)),
          new anchor.BN(assets[1].amount).sub(new anchor.BN(publicSplAmount)),
        ],
        account: senderAccount,
      });

      return [sendUtxo, changeUtxo];
    } else {
      const utxos: Utxo[] = [];
      assets.slice(1).forEach((asset, i) => {
        if (i === assets.slice(1).length - 1) {
          // add feeAsset as asset to the last spl utxo
          const utxo1 = new Utxo({
            poseidon,
            assets: [assets[0].asset, asset.asset],
            amounts: [
              // only implemented for shield! assumes passed in only if needed
              action !== Action.UNSHIELD
                ? new anchor.BN(publicSolAmount + assets[0].amount)
                : new anchor.BN(assets[0].amount),
              new anchor.BN(asset.amount),
            ],
            account: senderAccount, // if not self, use pubkey init // TODO: transfer: 1st is always recipient, 2nd change, both split sol min + rem to self
          });
          utxos.push(utxo1);
        } else {
          const utxo1 = new Utxo({
            poseidon,
            assets: [assets[0].asset, asset.asset],
            amounts: [new anchor.BN(0), new anchor.BN(asset.amount)],
            account: senderAccount, // if not self, use pubkey init
          });
          utxos.push(utxo1);
        }
      });
      if (utxos.length > 2)
        // TODO: implement for 3 assets (SPL,SPL,SOL)
        throw new Error(`Too many assets for outUtxo: ${assets.length}`);

      return utxos;
    }
  }*/
}
