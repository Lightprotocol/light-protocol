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
  POOL_TYPE,
  MINT,
  REGISTERED_POOL_PDA_SOL,
  verifierProgramZeroProgramId,
  verifierProgramOneProgramId,
  verifierProgramTwoProgramId,
  verifierProgramStorageProgramId,
  MESSAGE_MERKLE_TREE_KEY,
  Transaction,
  merkleTreeProgramId,
  airdropSol,
} from "../index";
import { MerkleTreeConfig } from "../merkleTree/merkleTreeConfig";

export async function setUpMerkleTree(provider: anchor.AnchorProvider) {
  var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
    TRANSACTION_MERKLE_TREE_KEY,
  );

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

    await merkleTreeConfig.registerPoolType(POOL_TYPE);
    console.log("Registering pool_type success");

    await merkleTreeConfig.registerSplPool(POOL_TYPE, MINT);
    console.log("Registering spl pool success");

    console.log("REGISTERED_POOL_PDA_SOL: ", REGISTERED_POOL_PDA_SOL);
    await merkleTreeConfig.registerSolPool(POOL_TYPE);
    console.log("Registering sol pool success");

    // TODO: do verifier registry in constants
    const verifierArray = [];
    verifierArray.push(
      new anchor.Program(
        IDL_VERIFIER_PROGRAM_ZERO,
        verifierProgramZeroProgramId,
      ),
    );
    verifierArray.push(
      new anchor.Program(IDL_VERIFIER_PROGRAM_ONE, verifierProgramOneProgramId),
    );
    verifierArray.push(
      new anchor.Program(IDL_VERIFIER_PROGRAM_TWO, verifierProgramTwoProgramId),
    );
    verifierArray.push(
      new anchor.Program(
        IDL_VERIFIER_PROGRAM_STORAGE,
        verifierProgramStorageProgramId,
      ),
    );
    // registering verifiers and airdrop sol to authority pdas
    for (var verifier of verifierArray) {
      await merkleTreeConfig.registerVerifier(verifier.programId);
      airdropSol({
        provider,
        amount: 1_000_000_000,
        recipientPublicKey: Transaction.getSignerAuthorityPda(
          merkleTreeProgramId,
          verifier.programId,
        ),
      });
      console.log(
        `Registering Verifier ${verifier.programId.toBase58()} and funded authority pda success`,
      );
    }
  }
}
