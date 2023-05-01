import { Program, Provider } from "@coral-xyz/anchor";
import {
  PublicKey,
  AddressLookupTableProgram,
  Keypair,
  SystemProgram,
  sendAndConfirmTransaction,
  Transaction,
  AccountInfo,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";
import { PathOrFileDescriptor, readFileSync, writeFile } from "fs";

import {
  MERKLE_TREE_KEY,
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  REGISTERED_POOL_PDA_SOL,
  DEFAULT_PROGRAMS,
  TOKEN_AUTHORITY,
  REGISTERED_POOL_PDA_SPL_TOKEN,
  PRE_INSERTED_LEAVES_INDEX,
  verifierProgramTwoProgramId,
  confirmConfig,
  verifierProgramZeroProgramId,
  merkleTreeProgramId,
  REGISTERED_VERIFIER_ONE_PDA,
  REGISTERED_VERIFIER_PDA,
  REGISTERED_VERIFIER_TWO_PDA,
  MINT,
} from "../index";
import { VerifierProgramZero, IDL_VERIFIER_PROGRAM_ZERO } from "../idls/index";

// TODO: create cli function to create a lookup table for apps
// Probably only works for testing
export async function initLookUpTableFromFile(
  provider: anchor.AnchorProvider,
  path: PathOrFileDescriptor = `lookUpTable.txt`,
  extraAccounts?: Array<PublicKey>,
) /*: Promise<PublicKey>*/ {
  const recentSlot = (await provider.connection.getSlot("confirmed")) - 10;

  const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;
  var [lookUpTable] = await PublicKey.findProgramAddress(
    [payerPubkey.toBuffer(), new anchor.BN(recentSlot).toBuffer("le", 8)],
    AddressLookupTableProgram.programId,
  );
  try {
    let lookUpTableRead = new PublicKey(readFileSync(path, "utf8"));
    let lookUpTableInfoInit = await provider.connection.getAccountInfo(
      lookUpTableRead,
    );
    if (lookUpTableInfoInit) {
      lookUpTable = lookUpTableRead;
    }
  } catch (e) {
    console.log(".txt not found", e);
  }

  let LOOK_UP_TABLE = await initLookUpTable(
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

export async function initLookUpTable(
  provider: Provider,
  lookupTableAddress: PublicKey,
  recentSlot: number,
  extraAccounts?: Array<PublicKey>,
): Promise<PublicKey> {
  var lookUpTableInfoInit: AccountInfo<Buffer> | null = null;
  if (lookupTableAddress != undefined) {
    lookUpTableInfoInit = await provider.connection.getAccountInfo(
      lookupTableAddress,
    );
  }

  if (lookUpTableInfoInit == null) {
    console.log("recentSlot: ", recentSlot);
    const payerPubkey = ADMIN_AUTH_KEYPAIR.publicKey;

    const createInstruction = AddressLookupTableProgram.createLookupTable({
      authority: payerPubkey,
      payer: payerPubkey,
      recentSlot,
    })[0];
    const verifierProgramZero: Program<VerifierProgramZero> = new Program(
      IDL_VERIFIER_PROGRAM_ZERO,
      verifierProgramZeroProgramId,
    );
    let escrows = (
      await PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("escrow")],
        verifierProgramZero.programId,
      )
    )[0];

    let ix0 = SystemProgram.transfer({
      fromPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      toPubkey: AUTHORITY,
      lamports: 1_000_000_0000,
    });

    var transaction = new Transaction().add(createInstruction);

    const addressesToAdd = [
      SystemProgram.programId,
      merkleTreeProgramId,
      DEFAULT_PROGRAMS.rent,
      MERKLE_TREE_KEY,
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
      lookupTable: lookupTableAddress,
      authority: payerPubkey,
      payer: payerPubkey,
      addresses: addressesToAdd,
    });

    transaction.add(extendInstruction);
    transaction.add(ix0);
    // transaction.add(ix1);
    let recentBlockhash = await provider.connection.getRecentBlockhash(
      "confirmed",
    );
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

    console.log("lookupTableAddress: ", lookupTableAddress.toBase58());
    let lookupTableAccount = await provider.connection.getAccountInfo(
      lookupTableAddress,
      "confirmed",
    );
    assert(lookupTableAccount != null);
  }
  return lookupTableAddress;
}
