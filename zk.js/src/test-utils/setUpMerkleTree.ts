import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import {
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  IDL_LIGHT_PSP2IN2OUT_STORAGE,
} from "../idls/index";

import {
  ADMIN_AUTH_KEYPAIR,
  POOL_TYPE,
  MINT,
  LegacyTransaction as Transaction,
  merkleTreeProgramId,
  airdropSol,
  lightPsp2in2outId,
  lightPsp10in2outId,
  lightPsp4in4outAppStorageId,
  lightPsp2in2outStorageId,
} from "../index";
import { MerkleTreeConfig } from "../merkleTree/merkleTreeConfig";

export async function setUpMerkleTree(
  provider: anchor.AnchorProvider,
  merkleTreeAuthority: PublicKey,
) {
  const merkleTreeConfig = new MerkleTreeConfig({
    payer: ADMIN_AUTH_KEYPAIR,
    anchorProvider: provider,
  });
  console.log(merkleTreeConfig.getMerkleTreeAuthorityPda());
  console.log(
    await provider.connection.getAccountInfo(
      merkleTreeConfig.getMerkleTreeAuthorityPda(),
    ),
  );

  if (
    (await provider.connection.getAccountInfo(
      merkleTreeConfig.getMerkleTreeAuthorityPda(),
    )) == null
  ) {
    await merkleTreeConfig.initMerkleTreeAuthority();
  } else {
    console.log("was already executed: initMerkleTreeAuthority");
  }

  if (
    (await provider.connection.getAccountInfo(
      (await merkleTreeConfig.getPoolTypePda(POOL_TYPE)).poolPda,
    )) == null
  ) {
    await merkleTreeConfig.registerPoolType(POOL_TYPE);
  } else {
    console.log("was already executed: registerPoolType");
  }

  if (
    (await provider.connection.getAccountInfo(
      (await merkleTreeConfig.getSplPoolPda(MINT, POOL_TYPE)).pda,
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
  const verifierArray: anchor.Program<any>[] = [];

  verifierArray.push(
    new anchor.Program(IDL_LIGHT_PSP2IN2OUT, lightPsp2in2outId),
  );
  verifierArray.push(
    new anchor.Program(IDL_LIGHT_PSP10IN2OUT, lightPsp10in2outId),
  );
  verifierArray.push(
    new anchor.Program(
      IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      lightPsp4in4outAppStorageId,
    ),
  );
  verifierArray.push(
    new anchor.Program(IDL_LIGHT_PSP2IN2OUT_STORAGE, lightPsp2in2outStorageId),
  );
  // registering verifiers and airdrop sol to authority pdas
  for (const verifier of verifierArray) {
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
      connection: provider.connection,
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
