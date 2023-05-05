import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  VerifierProgramOne,
  IDL_VERIFIER_PROGRAM_ONE,
  VerifierProgramTwo,
  IDL_VERIFIER_PROGRAM_TWO,
  VerifierProgramZero,
  IDL_VERIFIER_PROGRAM_ZERO,
  VerifierProgramStorage,
  IDL_VERIFIER_PROGRAM_STORAGE,
} from "../idls/index";

import {
  TRANSACTION_MERKLE_TREE_KEY,
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT_PRIVATE_KEY,
  POOL_TYPE,
  MINT,
  REGISTERED_POOL_PDA_SPL,
  REGISTERED_POOL_PDA_SOL,
  MERKLE_TREE_AUTHORITY_PDA,
  confirmConfig,
  verifierProgramZeroProgramId,
  verifierProgramOneProgramId,
  verifierProgramTwoProgramId,
  verifierStorageProgramId,
  MESSAGE_MERKLE_TREE_KEY,
} from "../index";
import { MerkleTreeConfig } from "../merkleTree/merkleTreeConfig";

export async function setUpMerkleTree(provider: anchor.AnchorProvider) {
  const verifierProgramZero: anchor.Program<VerifierProgramZero> =
    new anchor.Program(IDL_VERIFIER_PROGRAM_ZERO, verifierProgramZeroProgramId);
  const verifierProgramOne: anchor.Program<VerifierProgramOne> =
    new anchor.Program(IDL_VERIFIER_PROGRAM_ONE, verifierProgramOneProgramId);
  const verifierProgramTwo: anchor.Program<VerifierProgramTwo> =
    new anchor.Program(IDL_VERIFIER_PROGRAM_TWO, verifierProgramTwoProgramId);
  const verifierProgramStorage: anchor.Program<VerifierProgramStorage> =
    new anchor.Program(IDL_VERIFIER_PROGRAM_STORAGE, verifierStorageProgramId);

  var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
    TRANSACTION_MERKLE_TREE_KEY,
  );
  // console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
  // console.log("MERKLE_TREE_KEY ", MERKLE_TREE_KEY);
  // console.log("ADMIN_AUTH_KEYPAIR ", ADMIN_AUTH_KEYPAIR);

  if (merkleTreeAccountInfoInit == null) {
    let merkleTreeConfig = new MerkleTreeConfig({
      messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
    });

    console.log("Initing MERKLE_TREE_AUTHORITY_PDA");

    const ix1 = await merkleTreeConfig.initMerkleTreeAuthority();
    console.log("initMerkleTreeAuthority success, ", ix1);
    // assert(await provider.connection.getTransaction(ix, {commitment:"confirmed"}) != null, "init failed");

    // console.log("AUTHORITY: ", AUTHORITY);

    // console.log("AUTHORITY: ", Array.from(AUTHORITY.toBytes()));

    // console.log(
    //   "verifierProgramZero.programId: ",
    //   Array.from(verifierProgramZero.programId.toBytes()),
    // );
    // console.log("MERKLE_TREE_KEY: ", MERKLE_TREE_KEY.toBase58());
    // console.log("MERKLE_TREE_KEY: ", Array.from(MERKLE_TREE_KEY.toBytes()));
    // console.log("MERKLE_TREE_PDA_TOKEN: ", MERKLE_TREE_PDA_TOKEN.toBase58())
    // console.log("MERKLE_TREE_PDA_TOKEN: ", Array.from(MERKLE_TREE_PDA_TOKEN.toBytes()))

    const ix2 = await merkleTreeConfig.initializeNewMessageMerkleTree();
    assert(
      (await provider.connection.getTransaction(ix2, {
        commitment: "confirmed",
      })) != null,
      "init failed",
    );

    const ix3 = await merkleTreeConfig.initializeNewTransactionMerkleTree();
    assert(
      (await provider.connection.getTransaction(ix3, {
        commitment: "confirmed",
      })) != null,
      "init failed",
    );

    console.log("Registering Verifiers");
    await merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
    console.log("Registering Verifier Zero success");

    await merkleTreeConfig.registerVerifier(verifierProgramOne.programId);
    console.log("Registering Verifier One success");

    await merkleTreeConfig.registerVerifier(verifierProgramTwo.programId);
    console.log("Registering Verifier Two success");

    await merkleTreeConfig.registerVerifier(verifierProgramStorage.programId);
    console.log("Registering Verifier Storage success");

    await merkleTreeConfig.registerPoolType(POOL_TYPE);
    console.log("Registering pool_type success");

    // console.log("MINT: ", MINT.toBase58());
    // console.log("POOL_TYPE_PDA: ", REGISTERED_POOL_PDA_SPL.toBase58());
    await merkleTreeConfig.registerSplPool(POOL_TYPE, MINT);
    console.log("Registering spl pool success");

    console.log("REGISTERED_POOL_PDA_SOL: ", REGISTERED_POOL_PDA_SOL);
    await merkleTreeConfig.registerSolPool(POOL_TYPE);
    console.log("Registering sol pool success");
  }
}
