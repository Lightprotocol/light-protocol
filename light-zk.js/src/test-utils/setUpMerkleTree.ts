import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
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

export async function setUpMerkleTree(
  provider: anchor.AnchorProvider,
  merkleTreeAuthority: PublicKey,
  force?: boolean,
) {
  let merkleTreeConfig = new MerkleTreeConfig({
    messageMerkleTreePubkey: MESSAGE_MERKLE_TREE_KEY,
    transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
    payer: ADMIN_AUTH_KEYPAIR,
    connection: provider.connection,
  });
  console.log(await merkleTreeConfig.getMerkleTreeAuthorityPda());
  console.log(
    await provider.connection.getAccountInfo(
      await merkleTreeConfig.getMerkleTreeAuthorityPda(),
    ),
  );

  if (
    (await provider.connection.getAccountInfo(
      await merkleTreeConfig.getMerkleTreeAuthorityPda(),
    )) == null
  ) {
    await merkleTreeConfig.initMerkleTreeAuthority();
  } else {
    console.log("was already executed: initMerkleTreeAuthority");
  }

  if (
    (await provider.connection.getAccountInfo(MESSAGE_MERKLE_TREE_KEY)) == null
  ) {
    await merkleTreeConfig.initializeNewMessageMerkleTree();
  } else {
    console.log("was already executed: initializeNewMessageMerkleTree");
  }

  if (
    (await provider.connection.getAccountInfo(TRANSACTION_MERKLE_TREE_KEY)) ==
    null
  ) {
    await merkleTreeConfig.initializeNewTransactionMerkleTree();
  } else {
    console.log("was already executed: initializeNewTransactionMerkleTree");
  }

  if (
    (await provider.connection.getAccountInfo(
      (
        await merkleTreeConfig.getPoolTypePda(POOL_TYPE)
      ).poolPda,
    )) == null
  ) {
    await merkleTreeConfig.registerPoolType(POOL_TYPE);
  } else {
    console.log("was already executed: registerPoolType");
  }

  if (
    (await provider.connection.getAccountInfo(
      (
        await merkleTreeConfig.getSplPoolPda(MINT, POOL_TYPE)
      ).pda,
    )) == null
  ) {
    await merkleTreeConfig.registerSplPool(POOL_TYPE, MINT);
  } else {
    console.log("was already executed: registerSplPool");
  }

  if (
    (await provider.connection.getAccountInfo(
      MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId, POOL_TYPE).pda,
    )) == null
  ) {
    await merkleTreeConfig.registerSolPool(POOL_TYPE);
  } else {
    console.log("was already executed: registerSolPool");
  }

  // TODO: do verifier registry in constants
  const verifierArray = [];
  verifierArray.push(
    new anchor.Program(IDL_VERIFIER_PROGRAM_ZERO, verifierProgramZeroProgramId),
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
    const pda = (
      await merkleTreeConfig.getRegisteredVerifierPda(verifier.programId)
    ).registeredVerifierPda;
    if ((await provider.connection.getAccountInfo(pda)) == null) {
      await merkleTreeConfig.registerVerifier(verifier.programId);
    } else {
      console.log(
        `verifier ${verifier.programId.toBase58()} is already initialized`,
      );
    }
    const authorityPda = Transaction.getSignerAuthorityPda(
      merkleTreeProgramId,
      verifier.programId,
    );
    await airdropSol({
      provider,
      lamports: 1_000_000_000,
      recipientPublicKey: authorityPda,
    });
    console.log(
      `Registering Verifier ${verifier.programId.toBase58()}, pda ${pda.toBase58()} and funded authority pda success ${authorityPda.toBase58()}`,
    );
  }
  if (merkleTreeAuthority) {
    await merkleTreeConfig.updateMerkleTreeAuthority(merkleTreeAuthority, true);
  }
}
