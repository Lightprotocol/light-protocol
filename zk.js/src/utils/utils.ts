import { BN } from "@coral-xyz/anchor";
import {
  AddressLookupTableProgram,
  Connection,
  PublicKey,
  Transaction,
  SystemProgram,
} from "@solana/web3.js";
import * as os from "os";
import { sha256 } from "@noble/hashes/sha256";
import { Decimal } from "decimal.js";
import { SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import * as anchor from "@coral-xyz/anchor";

import { TokenData } from "../types";
import {
  AUTHORITY,
  DEFAULT_PROGRAMS,
  MERKLE_TREE_AUTHORITY_PDA,
  merkleTreeProgramId,
  PRE_INSERTED_LEAVES_INDEX,
  REGISTERED_POOL_PDA_SOL,
  REGISTERED_POOL_PDA_SPL_TOKEN,
  REGISTERED_VERIFIER_ONE_PDA,
  REGISTERED_VERIFIER_PDA,
  REGISTERED_VERIFIER_TWO_PDA,
  TOKEN_AUTHORITY,
  lightPsp2in2outId,
  lightPsp4in4outAppStorageId,
  BN_0,
  BN_1,
} from "../constants";
import { MerkleTreeConfig } from "../merkle-tree";
import { MINT } from "../test-utils/constants-system-verifier";
import { UtilsError, UtilsErrorCode } from "../errors";
import { Wallet } from "../provider";

import { Utxo } from "../utxo";
import { TokenUtxoBalance } from "../build-balance";

const crypto = require("@noble/hashes/crypto");

export function hashAndTruncateToCircuit(data: Uint8Array) {
  return truncateToCircuit(sha256.create().update(Buffer.from(data)).digest());
}

/**
 * Truncates the given 32-byte array to a 31-byte one, ensuring it fits
 * into the Fr modulo field.
 *
 * ## Safety
 *
 * This function is primarily used for truncating hashes (e.g., SHA-256) which are
 * not constrained by any modulo space. It's important to note that, as of now,
 * it's not possible to use any ZK-friendly function within a single transaction.
 * While truncating hashes to 31 bytes is generally safe, you should ensure that
 * this operation is appropriate for your specific use case.
 *
 * @param bytes The 32-byte array to be truncated.
 * @returns The truncated 31-byte array.
 *
 * @example
 * ```typescript
 * // example usage of truncate function
 * const truncated = truncateFunction(original32BytesArray);
 * ```
 */
export function truncateToCircuit(digest: Uint8Array) {
  return new BN(digest.slice(1, 32), undefined, "be");
}

// TODO: add pooltype
export async function getAssetLookUpId({
  anchorProvider,
  asset,
}: {
  asset: PublicKey;
  anchorProvider: anchor.AnchorProvider;
  // poolType?: Uint8Array
}): Promise<any> {
  const poolType = new Array(32).fill(0);
  const mtConf = new MerkleTreeConfig({
    anchorProvider,
  });
  const pubkey = await mtConf.getSplPoolPda(asset, poolType);

  const registeredAssets =
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

export function isEqualUint8Array(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) {
    return false;
  }
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) {
      return false;
    }
  }
  return true;
}

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
      : BN_0;
  } else {
    publicAmountSpl = publicAmountSpl ? new BN(publicAmountSpl) : undefined;
    publicAmountSol = publicAmountSol ? new BN(publicAmountSol) : BN_0;
  }
  return { publicAmountSpl, publicAmountSol };
}

export const convertAndComputeDecimals = (
  amount: BN | string | number,
  decimals: BN,
): BN => {
  if (typeof amount === "number" && amount < 0) {
    throw new UtilsError(
      UtilsErrorCode.INVALID_NUMBER,
      "decimalConversion",
      "Negative amounts are not allowed.",
    );
  }

  if (typeof amount === "string" && amount.startsWith("-")) {
    throw new UtilsError(
      UtilsErrorCode.INVALID_NUMBER,
      "decimalConversion",
      "Negative amounts are not allowed.",
    );
  }
  if (decimals.lt(BN_1)) {
    throw new UtilsError(
      UtilsErrorCode.INVALID_NUMBER,
      "decimalConversion",
      "Decimal numbers have to be at least 1 since we precompute 10**decimalValue.",
    );
  }

  const amountStr = amount.toString();

  if (!new Decimal(amountStr).isInt()) {
    const convertedFloat = new Decimal(amountStr).times(
      new Decimal(decimals.toString()),
    );
    if (!convertedFloat.isInt())
      throw new UtilsError(
        UtilsErrorCode.INVALID_NUMBER,
        "decimalConversion",
        `Decimal conversion of value ${amountStr} failed`,
      );
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
      new anchor.BN(nullifier.toString()).toArrayLike(Buffer, "be", 32),
      anchor.utils.bytes.utf8.encode("nf"),
    ],
    merkleTreeProgramId,
  )[0];
  let retries = 2;
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

  const accountObject: Partial<T> = {};
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
  if (!idl.accounts)
    throw new UtilsError(
      UtilsErrorCode.ACCOUNTS_UNDEFINED,
      "isProgramVerifier",
      "Idl does not contain accounts",
    );
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

  const [lookUpTable] = PublicKey.findProgramAddressSync(
    [
      payerPubkey.toBuffer(),
      new anchor.BN(recentSlot).toArrayLike(Buffer, "le", 8),
    ],
    AddressLookupTableProgram.programId,
  );

  const createInstruction = AddressLookupTableProgram.createLookupTable({
    authority: payerPubkey,
    payer: payerPubkey,
    recentSlot,
  })[0];

  const escrows = PublicKey.findProgramAddressSync(
    [anchor.utils.bytes.utf8.encode("escrow")],
    lightPsp2in2outId,
  )[0];

  const transaction = new Transaction().add(createInstruction);

  const addressesToAdd = [
    SystemProgram.programId,
    merkleTreeProgramId,
    DEFAULT_PROGRAMS.rent,
    SPL_NOOP_PROGRAM_ID,
    MERKLE_TREE_AUTHORITY_PDA,
    MerkleTreeConfig.getEventMerkleTreePda(),
    MerkleTreeConfig.getTransactionMerkleTreePda(),
    MerkleTreeConfig.getTransactionMerkleTreePda(new anchor.BN(1)),
    MerkleTreeConfig.getTransactionMerkleTreePda(new anchor.BN(2)),
    PRE_INSERTED_LEAVES_INDEX,
    AUTHORITY,
    TOKEN_PROGRAM_ID,
    escrows,
    TOKEN_AUTHORITY,
    REGISTERED_POOL_PDA_SOL,
    REGISTERED_POOL_PDA_SPL_TOKEN,
    lightPsp4in4outAppStorageId,
    REGISTERED_VERIFIER_ONE_PDA,
    REGISTERED_VERIFIER_PDA,
    REGISTERED_VERIFIER_TWO_PDA,
    MINT,
  ];

  if (extraAccounts) {
    for (const i in extraAccounts) {
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

  const recentBlockhash =
    await provider.connection.getLatestBlockhash("confirmed");
  transaction.feePayer = payerPubkey;
  transaction.recentBlockhash = recentBlockhash.blockhash;

  try {
    await payer.sendAndConfirmTransaction(transaction);
  } catch (e) {
    throw new UtilsError(
      UtilsErrorCode.LOOK_UP_TABLE_CREATION_FAILED,
      "initLookUpTable",
      `Creating lookup table failed payer: ${payerPubkey}, transaction ${JSON.stringify(
        transaction,
      )}, error ${e}`,
    );
  }

  const lookupTableAccount = await provider.connection.getAccountInfo(
    lookUpTable,
    "confirmed",
  );
  if (lookupTableAccount == null)
    throw new UtilsError(
      UtilsErrorCode.LOOK_UP_TABLE_CREATION_FAILED,
      "initLookUpTable",
      `Creating lookup table failed payer: ${payerPubkey}`,
    );
  return lookUpTable;
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

export enum System {
  MacOsAmd64,
  MacOsArm64,
  LinuxAmd64,
  LinuxArm64,
}

export function getSystem(): System {
  const arch = os.arch();
  const platform = os.platform();

  switch (platform) {
    case "darwin":
      switch (arch) {
        case "x64":
          return System.MacOsAmd64;
        case "arm":
        // fallthrough
        case "arm64":
          return System.MacOsArm64;
        default:
          throw new UtilsError(
            UtilsErrorCode.UNSUPPORTED_ARCHITECTURE,
            "getSystem",
            `Architecture ${arch} is not supported.`,
          );
      }
    case "linux":
      switch (arch) {
        case "x64":
          return System.LinuxAmd64;
        case "arm":
        // fallthrough
        case "arm64":
          return System.LinuxArm64;
        default:
          throw new UtilsError(
            UtilsErrorCode.UNSUPPORTED_ARCHITECTURE,
            "getSystem",
            `Architecture ${arch} is not supported.`,
          );
      }
  }

  throw new UtilsError(
    UtilsErrorCode.UNSUPPORTED_PLATFORM,
    "getSystem",
    `Platform ${platform} is not supported.`,
  );
}
