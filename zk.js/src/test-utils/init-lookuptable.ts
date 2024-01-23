import { Program } from "@coral-xyz/anchor";
import {
  PublicKey,
  AddressLookupTableProgram,
  SystemProgram,
  sendAndConfirmTransaction,
  Transaction,
  AccountInfo,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { PathLike, readFileSync, writeFile } from "fs";

import { LightPsp2in2out, IDL_LIGHT_PSP2IN2OUT } from "../idls";
import { SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";
import { ADMIN_AUTH_KEYPAIR, MINT } from "./constants-system-verifier";
import { MerkleTreeConfig } from "../merkle-tree";
import {
  AUTHORITY,
  REGISTERED_POOL_PDA_SOL,
  DEFAULT_PROGRAMS,
  TOKEN_AUTHORITY,
  REGISTERED_POOL_PDA_SPL_TOKEN,
  PRE_INSERTED_LEAVES_INDEX,
  lightPsp4in4outAppStorageId,
  confirmConfig,
  lightPsp2in2outId,
  merkleTreeProgramId,
  REGISTERED_VERIFIER_ONE_PDA,
  REGISTERED_VERIFIER_PDA,
  REGISTERED_VERIFIER_TWO_PDA,
} from "../constants";

// TODO: create cli function to create a lookup table for apps
// Probably only works for testing
export async function initLookUpTableFromFile(
  provider: anchor.AnchorProvider,
  path: PathLike = `lookUpTable.txt`,
  extraAccounts?: Array<PublicKey>,
) /*: Promise<PublicKey>*/ {
  const recentSlot = (await provider.connection.getSlot("confirmed")) - 10;

  const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;
  let [lookUpTable] = PublicKey.findProgramAddressSync(
    [
      payerPubkey.toBuffer(),
      new anchor.BN(recentSlot).toArrayLike(Buffer, "le", 8),
    ],
    AddressLookupTableProgram.programId,
  );
  try {
    const lookUpTableRead = new PublicKey(readFileSync(path, "utf8"));
    const lookUpTableInfoInit =
      await provider.connection.getAccountInfo(lookUpTableRead);
    if (lookUpTableInfoInit) {
      lookUpTable = lookUpTableRead;
    }
  } catch (_) {
    /* empty */
  }

  const LOOK_UP_TABLE = await initLookUpTableTest(
    provider,
    lookUpTable,
    recentSlot,
    extraAccounts,
  );
  writeFile(path, LOOK_UP_TABLE.toString(), function (err) {
    if (err) {
      return console.error(err);
    }
  });

  return LOOK_UP_TABLE; //new Promise((resolveOuter) => {LOOK_UP_TABLE});
}

export async function initLookUpTableTest(
  provider: anchor.AnchorProvider,
  lookupTableAddress: PublicKey,
  recentSlot: number,
  extraAccounts: Array<PublicKey> = [],
): Promise<PublicKey> {
  let lookUpTableInfoInit: AccountInfo<Buffer> | null = null;
  if (lookupTableAddress != undefined) {
    lookUpTableInfoInit =
      await provider.connection.getAccountInfo(lookupTableAddress);
  }

  if (lookUpTableInfoInit == null) {
    console.log("recentSlot: ", recentSlot);
    const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;

    const createInstruction = AddressLookupTableProgram.createLookupTable({
      authority: payerPubkey,
      payer: payerPubkey,
      recentSlot,
    })[0];
    const lightPsp2in2out: Program<LightPsp2in2out> = new Program(
      IDL_LIGHT_PSP2IN2OUT,
      lightPsp2in2outId,
      provider,
    );
    const escrows = (
      await PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("escrow")],
        lightPsp2in2out.programId,
      )
    )[0];

    const ix0 = SystemProgram.transfer({
      fromPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      toPubkey: AUTHORITY,
      lamports: 1_000_000_0000,
    });

    const transaction = new Transaction().add(createInstruction);

    const addressesToAdd = [
      SystemProgram.programId,
      merkleTreeProgramId,
      DEFAULT_PROGRAMS.rent,
      SPL_NOOP_PROGRAM_ID,
      MerkleTreeConfig.getEventMerkleTreePda(),
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      PRE_INSERTED_LEAVES_INDEX,
      AUTHORITY,
      TOKEN_PROGRAM_ID,
      escrows,
    ];
    const additonalAccounts = [
      TOKEN_AUTHORITY,
      REGISTERED_POOL_PDA_SOL,
      REGISTERED_POOL_PDA_SPL_TOKEN,
      lightPsp4in4outAppStorageId,
      REGISTERED_VERIFIER_ONE_PDA,
      REGISTERED_VERIFIER_PDA,
      REGISTERED_VERIFIER_TWO_PDA,
      MINT,
    ];
    extraAccounts = extraAccounts.concat(additonalAccounts);
    if (extraAccounts) {
      for (const i in extraAccounts) {
        addressesToAdd.push(extraAccounts[i]);
      }
    }

    const extendInstruction = AddressLookupTableProgram.extendLookupTable({
      lookupTable: lookupTableAddress,
      authority: payerPubkey,
      payer: payerPubkey,
      addresses: addressesToAdd,
    });

    transaction.add(extendInstruction);
    transaction.add(ix0);
    const recentBlockhash =
      await provider.connection.getLatestBlockhash("confirmed");
    transaction.feePayer = payerPubkey;
    transaction.recentBlockhash = recentBlockhash.blockhash;

    try {
      await sendAndConfirmTransaction(
        provider.connection,
        transaction,
        [ADMIN_AUTH_KEYPAIR],
        confirmConfig,
      );
    } catch (e) {
      console.log("e : ", e);
    }

    const lookupTableAccount = await provider.connection.getAccountInfo(
      lookupTableAddress,
      "confirmed",
    );
    if (!lookupTableAccount) {
      throw new Error("lookupTableAccount is null or undefined");
    }
  }
  return lookupTableAddress;
}
