import { BN } from "@coral-xyz/anchor";
import {
  confirmConfig,
  merkleTreeProgram,
  merkleTreeProgramId,
  MESSAGE_MERKLE_TREE_KEY,
  TRANSACTION_MERKLE_TREE_KEY,
} from "./constants";
import { Connection, PublicKey, SystemProgram } from "@solana/web3.js";
import { MerkleTreeConfig, SolMerkleTree } from "./merkleTree";
import { MINT } from "./test-utils/constants_system_verifier";
import * as anchor from "@coral-xyz/anchor";
import { Utxo } from "utxo";
import { MetaError, UtilsError, UtilsErrorCode } from "./errors";
import { TokenUtxoBalance } from "wallet";
const { keccak_256 } = require("@noble/hashes/sha3");

export function hashAndTruncateToCircuit(data: Uint8Array) {
  return new BN(
    keccak_256
      .create({ dkLen: 32 })
      .update(Buffer.from(data))
      .digest()
      .slice(1, 32),
    undefined,
    "be",
  );
}

// TODO: add pooltype
export async function getAssetLookUpId({
  connection,
  asset,
}: {
  asset: PublicKey;
  connection: Connection;
  // poolType?: Uint8Array
}): Promise<any> {
  let poolType = new Array(32).fill(0);
  let mtConf = new MerkleTreeConfig({
    connection,
    messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
    transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
  });
  let pubkey = await mtConf.getSplPoolPda(asset, poolType);

  let registeredAssets =
    await mtConf.merkleTreeProgram.account.registeredAssetPool.fetch(
      pubkey.pda,
    );
  return registeredAssets.index;
}

// TODO: fetch from chain
// TODO: separate testing variables from prod env
export const assetLookupTable: string[] = [
  SystemProgram.programId.toBase58(),
  MINT.toBase58(),
];

export const verifierLookupTable: string[] = [
  SystemProgram.programId.toBase58(),
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
];

export function getAssetIndex(assetPubkey: PublicKey): BN {
  return new BN(assetLookupTable.indexOf(assetPubkey.toBase58()));
}

export function fetchAssetByIdLookUp(assetIndex: BN): PublicKey {
  return new PublicKey(assetLookupTable[assetIndex.toNumber()]);
}

export function fetchVerifierByIdLookUp(index: BN): PublicKey {
  return new PublicKey(verifierLookupTable[index.toNumber()]);
}

export const arrToStr = (uint8arr: Uint8Array) =>
  "LPx" + Buffer.from(uint8arr.buffer).toString("hex");

export const strToArr = (str: string) =>
  new Uint8Array(Buffer.from(str.slice(3), "hex"));

export const convertAndComputeDecimals = (
  amount: BN | string | number,
  decimals: BN,
) => {
  return new BN(amount.toString()).mul(decimals);
};

export const getUpdatedSpentUtxos = (
  tokenBalances: Map<string, TokenUtxoBalance>,
): Utxo[] => {
  return Array.from(tokenBalances.values())
    .map((value) => Array.from(value.spentUtxos.values()))
    .flat();
};

export const fetchNullifierAccountInfo = async (
  nullifier: string,
  connection: Connection,
) => {
  const nullifierPubkey = PublicKey.findProgramAddressSync(
    [
      Buffer.from(new anchor.BN(nullifier.toString()).toArray()),
      anchor.utils.bytes.utf8.encode("nf"),
    ],
    merkleTreeProgramId,
  )[0];
  return connection.getAccountInfo(nullifierPubkey, "confirmed");
};

// use
export const fetchQueuedLeavesAccountInfo = async (
  leftLeaf: Uint8Array,
  connection: Connection,
) => {
  const queuedLeavesPubkey = PublicKey.findProgramAddressSync(
    [leftLeaf, anchor.utils.bytes.utf8.encode("leaves")],
    merkleTreeProgramId,
  )[0];
  return connection.getAccountInfo(queuedLeavesPubkey, "confirmed");
};

export const sleep = (ms: number) => {
  return new Promise((resolve) => setTimeout(resolve, ms));
};

// export var logger = (function () {
//   var oldConsoleLog: any = null;
//   var pub = {};

//   //@ts-ignore
//   pub.enableLogger = function enableLogger() {
//     if (oldConsoleLog == null) return;

//     console.log = oldConsoleLog;
//   };

//   //@ts-ignore
//   pub.disableLogger = function disableLogger() {
//     oldConsoleLog = console.log;
//     window["console"]["log"] = function () {};
//   };

//   return pub;
// })();

export type KeyValue = {
  [key: string]: any;
};
/**
 * @description Creates an object of a type defined in accounts[accountName],
 * @description all properties need to be part of obj, if a property is missing an error is thrown.
 * @description The accounts array is part of an anchor idl.
 * @param obj Object properties are picked from.
 * @param accounts Idl accounts array from which accountName is selected.
 * @param accountName Defines which account in accounts to use as type for the output object.
 * @returns
 */
export function createAccountObject<T extends KeyValue>(
  obj: T,
  accounts: any[],
  accountName: string,
): Partial<KeyValue> {
  const account = accounts.find((account) => account.name === accountName);

  if (!account) {
    throw new UtilsError(
      UtilsErrorCode.ACCOUNT_NAME_UNDEFINED_IN_IDL,
      "pickFieldsFromObject",
      `${accountName} does not exist in idl`,
    );
  }

  const fieldNames = account.type.fields.map(
    (field: { name: string }) => field.name,
  );

  let accountObject: Partial<T> = {};
  fieldNames.forEach((fieldName: keyof T) => {
    accountObject[fieldName] = obj[fieldName];
    if (!accountObject[fieldName])
      throw new UtilsError(
        UtilsErrorCode.PROPERY_UNDEFINED,
        "pickFieldsFromObject",
        `Property ${fieldName.toString()} undefined`,
      );
  });
  return accountObject;
}
