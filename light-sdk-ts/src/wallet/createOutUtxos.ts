import { PublicKey, SystemProgram } from "@solana/web3.js";
import { FEE_ASSET } from "../constants";
import { Relayer } from "../relayer";
import { Utxo } from "../utxo";
import * as anchor from "@coral-xyz/anchor";
import { Account } from "../account";
// TODO: v4: handle custom outCreator fns -> oututxo[] can be passed as param
export function createOutUtxos({
  mint,
  amount,
  inUtxos,
  recipient,
  recipientEncryptionPublicKey,
  relayer,
  extraSolAmount,
  poseidon,
  account,
}: {
  mint: PublicKey;
  amount: number;
  inUtxos: Utxo[];
  recipient?: anchor.BN;
  recipientEncryptionPublicKey?: Uint8Array;
  relayer?: Relayer;
  extraSolAmount: number;
  poseidon: any;
  account: Account;
}) {
  //   const { poseidon } = provider;
  if (!poseidon) throw new Error("Poseidon not initialized");
  if (!account) throw new Error("Shielded Account not initialized");

  if (amount < 0) {
    let inAmount = 0;
    inUtxos.forEach((inUtxo) => {
      inUtxo.assets.forEach((asset, i) => {
        if (asset.toBase58() === mint.toBase58()) {
          inAmount += inUtxo.amounts[i].toNumber();
        }
      });
    });
    if (inAmount < Math.abs(amount)) {
      throw new Error(
        `Insufficient funds for unshield/transfer. In amount: ${inAmount}, out amount: ${amount}`,
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
    assets.push({ asset: mint, amount: !isTransfer ? amount : 0 });
  } else {
    assets[assetIndex].amount += !isTransfer ? amount : 0;
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
        amounts: [new anchor.BN(amount)],
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
            .sub(new anchor.BN(amount))
            .sub(relayer?.relayerFee || new anchor.BN(0)), // sub from change
          assets[1] ? new anchor.BN(assets[1].amount) : new anchor.BN(0),
        ], // rem transfer positive
        account: account,
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
            ? new anchor.BN(extraSolAmount + assets[0].amount)
            : new anchor.BN(assets[0].amount),
          assets[1] ? new anchor.BN(assets[1].amount) : new anchor.BN(0),
        ],
        account: recipient
          ? new Account({
              poseidon: poseidon,
              publicKey: recipient,
              encryptionPublicKey: recipientEncryptionPublicKey,
            })
          : account, // if not self, use pubkey init
      });

      return [feeAssetChangeUtxo];
    }
  } else {
    if (isTransfer) {
      let sendAmountFeeAsset = new anchor.BN(1e5);

      let sendUtxo = new Utxo({
        poseidon,
        assets: [assets[0].asset, assets[1].asset],
        amounts: [sendAmountFeeAsset, new anchor.BN(amount)],
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
          new anchor.BN(assets[1].amount).sub(new anchor.BN(amount)),
        ],
        account: account,
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
                ? new anchor.BN(extraSolAmount + assets[0].amount)
                : new anchor.BN(assets[0].amount),
              new anchor.BN(asset.amount),
            ],
            account: account, // if not self, use pubkey init // TODO: transfer: 1st is always recipient, 2nd change, both split sol min + rem to self
          });
          utxos.push(utxo1);
        } else {
          const utxo1 = new Utxo({
            poseidon,
            assets: [assets[0].asset, asset.asset],
            amounts: [new anchor.BN(0), new anchor.BN(asset.amount)],
            account: account, // if not self, use pubkey init
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
