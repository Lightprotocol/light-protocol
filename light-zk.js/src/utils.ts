import { BN } from "@coral-xyz/anchor";
import {
  AUTHORITY,
  confirmConfig,
  DEFAULT_PROGRAMS,
  merkleTreeProgram,
  merkleTreeProgramId,
  MESSAGE_MERKLE_TREE_KEY,
  PRE_INSERTED_LEAVES_INDEX,
  REGISTERED_POOL_PDA_SOL,
  REGISTERED_POOL_PDA_SPL_TOKEN,
  REGISTERED_VERIFIER_ONE_PDA,
  REGISTERED_VERIFIER_PDA,
  REGISTERED_VERIFIER_TWO_PDA,
  TOKEN_AUTHORITY,
  TRANSACTION_MERKLE_TREE_KEY,
  verifierProgramTwoProgramId,
  verifierProgramZeroProgramId,
} from "./constants";
import {
  AccountInfo,
  AddressLookupTableProgram,
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
  Transaction,
  SystemProgram,
} from "@solana/web3.js";
import { MerkleTreeConfig, SolMerkleTree } from "./merkleTree";
import { MINT } from "./test-utils/constants_system_verifier";
import * as anchor from "@coral-xyz/anchor";
import { Utxo } from "./utxo";
import { MetaError, UtilsError, UtilsErrorCode } from "./errors";
import { TokenUtxoBalance, Wallet } from "./wallet";
import { TokenData } from "./types";
const { keccak_256 } = require("@noble/hashes/sha3");
import { Decimal } from "decimal.js";
import { SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
const crypto = require("@noble/hashes/crypto");

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

export function getAssetIndex(
  assetPubkey: PublicKey,
  assetLookupTable: string[],
): BN {
  return new BN(assetLookupTable.indexOf(assetPubkey.toBase58()));
}

export function fetchAssetByIdLookUp(
  assetIndex: BN,
  assetLookupTable: string[],
): PublicKey {
  return new PublicKey(assetLookupTable[assetIndex.toNumber()]);
}

export function fetchVerifierByIdLookUp(
  index: BN,
  verifierProgramLookupTable: string[],
): PublicKey {
  return new PublicKey(verifierProgramLookupTable[index.toNumber()]);
}

export const arrToStr = (uint8arr: Uint8Array) =>
  "LPx" + Buffer.from(uint8arr.buffer).toString("hex");

export const strToArr = (str: string) =>
  new Uint8Array(Buffer.from(str.slice(3), "hex"));

export function decimalConversion({
  tokenCtx,
  skipDecimalConversions,
  publicAmountSpl,
  publicAmountSol,
  minimumLamports,
  minimumLamportsAmount,
}: {
  tokenCtx: TokenData;
  skipDecimalConversions?: boolean;
  publicAmountSpl?: BN | string | number;
  publicAmountSol?: BN | string | number;
  minimumLamports?: boolean;
  minimumLamportsAmount?: BN;
}) {
  if (!skipDecimalConversions) {
    publicAmountSpl = publicAmountSpl
      ? convertAndComputeDecimals(publicAmountSpl, tokenCtx.decimals)
      : undefined;
    // If SOL amount is not provided, the default value is either minimum amount (if defined) or 0.
    publicAmountSol = publicAmountSol
      ? convertAndComputeDecimals(publicAmountSol, new BN(1e9))
      : minimumLamports
      ? minimumLamportsAmount
      : new BN(0);
  } else {
    publicAmountSpl = publicAmountSpl
      ? new BN(publicAmountSpl.toString())
      : undefined;
    publicAmountSol = publicAmountSol
      ? new BN(publicAmountSol?.toString())
      : new BN(0);
  }
  return { publicAmountSpl, publicAmountSol };
}
export const convertAndComputeDecimals = (
  amount: BN | string | number,
  decimals: BN,
) => {
  if (typeof amount === "number" && amount < 0) {
    throw new Error("Negative amounts are not allowed.");
  }

  if (typeof amount === "string" && amount.startsWith("-")) {
    throw new Error("Negative amounts are not allowed.");
  }
  if (decimals.lt(new BN(1))) {
    throw new Error(
      "Decimal numbers have to be at least 1 since we precompute 10**decimalValue.",
    );
  }

  let amountStr = amount.toString();

  if (!new Decimal(amountStr).isInt()) {
    const convertedFloat = new Decimal(amountStr).times(
      new Decimal(decimals.toString()),
    );
    if (!convertedFloat.isInt())
      throw new Error(`Decimal conversion of value ${amountStr} failed`);
    return new BN(convertedFloat.toString());
  }

  const bnAmount = new BN(amountStr);
  return bnAmount.mul(decimals);
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
      new anchor.BN(nullifier.toString()).toBuffer("be", 32),
      anchor.utils.bytes.utf8.encode("nf"),
    ],
    merkleTreeProgramId,
  )[0];
  var retries = 2;
  while (retries > 0) {
    const res = await connection.getAccountInfo(nullifierPubkey, "processed");
    if (res) return res;
    retries--;
  }
  return connection.getAccountInfo(nullifierPubkey, "processed");
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
        UtilsErrorCode.PROPERTY_UNDEFINED,
        "pickFieldsFromObject",
        `Property ${fieldName.toString()} undefined`,
      );
  });
  return accountObject;
}

export function firstLetterToLower(input: string): string {
  if (!input) return input;
  return input.charAt(0).toLowerCase() + input.slice(1);
}

export function firstLetterToUpper(input: string): string {
  if (!input) return input;
  return input.charAt(0).toUpperCase() + input.slice(1);
}

/**
 * This function checks if an account in the provided idk object exists with a name
 * ending with 'PublicInputs' and contains a field named 'publicAppVerifier'.
 *
 * @param {Idl} idl - The IDL object to check.
 * @returns {boolean} - Returns true if such an account exists, false otherwise.
 */
export function isProgramVerifier(idl: anchor.Idl): boolean {
  if (!idl.accounts) throw new Error("Idl does not contain accounts");
  return idl.accounts.some(
    (account) =>
      account.name.endsWith("PublicInputs") &&
      account.type.fields.some((field) => field.name === "publicAppVerifier"),
  );
}

export async function initLookUpTable(
  payer: Wallet,
  provider: anchor.Provider,
  extraAccounts?: Array<PublicKey>,
): Promise<PublicKey> {
  const payerPubkey = payer.publicKey;
  const recentSlot = (await provider.connection.getSlot("confirmed")) - 10;

  var [lookUpTable] = await PublicKey.findProgramAddressSync(
    [payerPubkey.toBuffer(), new anchor.BN(recentSlot).toBuffer("le", 8)],
    AddressLookupTableProgram.programId,
  );

  const createInstruction = AddressLookupTableProgram.createLookupTable({
    authority: payerPubkey,
    payer: payerPubkey,
    recentSlot,
  })[0];

  let escrows = (
    await PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("escrow")],
      verifierProgramZeroProgramId,
    )
  )[0];

  var transaction = new Transaction().add(createInstruction);

  const addressesToAdd = [
    SystemProgram.programId,
    merkleTreeProgramId,
    DEFAULT_PROGRAMS.rent,
    SPL_NOOP_PROGRAM_ID,
    MESSAGE_MERKLE_TREE_KEY,
    TRANSACTION_MERKLE_TREE_KEY,
    PRE_INSERTED_LEAVES_INDEX,
    AUTHORITY,
    TOKEN_PROGRAM_ID,
    escrows,
    TOKEN_AUTHORITY,
    REGISTERED_POOL_PDA_SOL,
    REGISTERED_POOL_PDA_SPL_TOKEN,
    verifierProgramTwoProgramId,
    REGISTERED_VERIFIER_ONE_PDA,
    REGISTERED_VERIFIER_PDA,
    REGISTERED_VERIFIER_TWO_PDA,
    MINT,
  ];

  if (extraAccounts) {
    for (var i in extraAccounts) {
      addressesToAdd.push(extraAccounts[i]);
    }
  }

  const extendInstruction = AddressLookupTableProgram.extendLookupTable({
    lookupTable: lookUpTable,
    authority: payerPubkey,
    payer: payerPubkey,
    addresses: addressesToAdd,
  });

  transaction.add(extendInstruction);

  let recentBlockhash = await provider.connection.getLatestBlockhash(
    "confirmed",
  );
  transaction.feePayer = payerPubkey;
  transaction.recentBlockhash = recentBlockhash.blockhash;

  try {
    await payer.sendAndConfirmTransaction(transaction);
  } catch (e) {
    console.log("e : ", e);
  }

  let lookupTableAccount = await provider.connection.getAccountInfo(
    lookUpTable,
    "confirmed",
  );
  if (lookupTableAccount == null)
    throw new Error("Creating lookup table failed");
  return lookUpTable;
}

export function toSnakeCase(str: string): string {
  return str.replace(/-/g, "_");
}

// setting environment correctly for ethereum-crypto
export function setEnvironment() {
  if (
    typeof process !== "undefined" &&
    process.versions != null &&
    process.versions.node != null
  ) {
    crypto.node = require("crypto");
  } else {
    crypto.web = window.crypto;
  }
}
