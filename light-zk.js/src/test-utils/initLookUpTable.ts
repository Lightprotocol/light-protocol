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
  TRANSACTION_MERKLE_TREE_KEY,
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
  MESSAGE_MERKLE_TREE_KEY,
} from "../index";
import { VerifierProgramZero, IDL_VERIFIER_PROGRAM_ZERO } from "../idls/index";
import { SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";

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

  let LOOK_UP_TABLE = await initLookUpTableTest(
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
  provider: Provider,
  lookupTableAddress: PublicKey,
  recentSlot: number,
  extraAccounts: Array<PublicKey> = [],
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
      SPL_NOOP_PROGRAM_ID,
      MESSAGE_MERKLE_TREE_KEY,
      TRANSACTION_MERKLE_TREE_KEY,
      PRE_INSERTED_LEAVES_INDEX,
      AUTHORITY,
      TOKEN_PROGRAM_ID,
      escrows,
    ];
    const additonalAccounts = [
      TOKEN_AUTHORITY,
      REGISTERED_POOL_PDA_SOL,
      REGISTERED_POOL_PDA_SPL_TOKEN,
      verifierProgramTwoProgramId,
      REGISTERED_VERIFIER_ONE_PDA,
      REGISTERED_VERIFIER_PDA,
      REGISTERED_VERIFIER_TWO_PDA,
      MINT,
    ];
    extraAccounts = extraAccounts.concat(additonalAccounts);
    if (extraAccounts) {
      for (var i in extraAccounts) {
        addressesToAdd.push(extraAccounts[i]);
      }
    }

    // const extendInstruction = AddressLookupTableProgram.extendLookupTable({
    //   lookupTable: lookupTableAddress,
    //   authority: payerPubkey,
    //   payer: payerPubkey,
    //   addresses: addressesToAdd,
    // });

    // transaction.add(extendInstruction);
    transaction.add(ix0);
    // transaction.add(ix1);
    let recentBlockhash = await provider.connection.getLatestBlockhash(
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

    let lookupTableAccount = await provider.connection.getAccountInfo(
      lookupTableAddress,
      "confirmed",
    );
    assert(lookupTableAccount != null);
  }
  return lookupTableAddress;
}
