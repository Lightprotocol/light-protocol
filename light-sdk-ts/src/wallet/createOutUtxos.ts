import { PublicKey, SystemProgram } from "@solana/web3.js";
import { FEE_ASSET } from "../constants";
import { Relayer } from "../relayer";
import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Account } from "../account";
import { Action } from "transaction";
// TODO: v4: handle custom outCreator fns -> oututxo[] can be passed as param

// I think this should be part of transaction parameters

// pass in action
// sanity checks for action
// change amounts to solAmount, splAmount
// pass in recipientAccount
// make userAccount explicit
//
export function createOutUtxos({
  mint,
  splAmount,
  inUtxos,
  recipient,
  recipientEncryptionPublicKey,
  relayer,
  solAmount,
  poseidon,
  senderAccount,
  action,
}: {
  mint: PublicKey;
  splAmount: number;
  inUtxos: Utxo[];
  recipient?: anchor.BN;
  recipientEncryptionPublicKey?: Uint8Array;
  relayer?: Relayer;
  solAmount: number;
  poseidon: any;
  senderAccount: Account;
  action: Action;
}) {
  //   const { poseidon } = provider;
  if (!poseidon) throw new Error("Poseidon not initialized");
  if (!senderAccount) throw new Error("Shielded Account not initialized");

  if (splAmount < 0) {
    let inAmount = 0;
    inUtxos.forEach((inUtxo) => {
      inUtxo.assets.forEach((asset, i) => {
        if (asset.toBase58() === mint.toBase58()) {
          inAmount += inUtxo.amounts[i].toNumber();
        }
      });
    });
    if (inAmount < Math.abs(splAmount)) {
      throw new Error(
        `Insufficient funds for unshield/transfer. In splAmount: ${inAmount}, out splAmount: ${splAmount}`,
      );
    }
  }
  var isTransfer = false;
  var isUnshield = false;
  if (recipient && recipientEncryptionPublicKey && relayer) isTransfer = true;

  if (!recipientEncryptionPublicKey && relayer) isUnshield = true;
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
    assets.push({ asset: mint, amount: !isTransfer ? splAmount : 0 });
  } else {
    assets[assetIndex].amount += !isTransfer ? splAmount : 0;
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

    if (isTransfer) {
      let feeAssetSendUtxo = new Utxo({
        poseidon,
        assets: [assets[0].asset],
        amounts: [new anchor.BN(solAmount)],
        account: new Account({
          poseidon: poseidon,
          publicKey: recipient,
          encryptionPublicKey: recipientEncryptionPublicKey,
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
            .sub(new anchor.BN(solAmount))
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
          !isUnshield
            ? new anchor.BN(solAmount + assets[0].amount)
            : new anchor.BN(assets[0].amount),
          assets[1] ? new anchor.BN(assets[1].amount) : new anchor.BN(0),
        ],
        account: recipient
          ? new Account({
              poseidon: poseidon,
              publicKey: recipient,
              encryptionPublicKey: recipientEncryptionPublicKey,
            })
          : senderAccount, // if not self, use pubkey init
      });

      return [feeAssetChangeUtxo];
    }
  } else {
    if (isTransfer) {
      let sendAmountFeeAsset = new anchor.BN(1e5);

      let sendUtxo = new Utxo({
        poseidon,
        assets: [assets[0].asset, assets[1].asset],
        amounts: [sendAmountFeeAsset, new anchor.BN(splAmount)],
        account: new Account({
          poseidon: poseidon,
          publicKey: recipient,
          encryptionPublicKey: recipientEncryptionPublicKey,
        }),
      });
      let changeUtxo = new Utxo({
        poseidon,
        assets: [assets[0].asset, assets[1].asset],
        amounts: [
          new anchor.BN(assets[0].amount)
            .sub(sendAmountFeeAsset)
            .sub(relayer?.relayerFee || new anchor.BN(0)),
          new anchor.BN(assets[1].amount).sub(new anchor.BN(splAmount)),
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
              !isUnshield
                ? new anchor.BN(solAmount + assets[0].amount)
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
  }
}
