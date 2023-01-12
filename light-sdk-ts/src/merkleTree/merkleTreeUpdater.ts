import * as anchor from "@coral-xyz/anchor";
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";
import {
  checkMerkleTreeUpdateStateCreated,
  checkMerkleTreeBatchUpdateSuccess,
} from "../test-utils/testChecks";

import { confirmConfig, DEFAULT_PROGRAMS } from "../index";
import {
  ComputeBudgetProgram,
  PublicKey,
  Transaction,
  SystemProgram,
  Keypair,
} from "@solana/web3.js";
import { publicKey } from "@coral-xyz/anchor/dist/cjs/utils";

export async function executeUpdateMerkleTreeTransactions({
  signer,
  merkleTreeProgram,
  leavesPdas,
  merkleTree,
  merkleTreeIndex,
  merkle_tree_pubkey,
  connection,
  provider,
}) {
  var merkleTreeAccountPrior = await merkleTreeProgram.account.merkleTree.fetch(
    merkle_tree_pubkey,
  );
  let merkleTreeUpdateState = (
    await PublicKey.findProgramAddressSync(
      [
        Buffer.from(new Uint8Array(signer.publicKey.toBytes())),
        anchor.utils.bytes.utf8.encode("storage"),
      ],
      merkleTreeProgram.programId,
    )
  )[0];

  try {
    const tx1 = await merkleTreeProgram.methods
      .initializeMerkleTreeUpdateState
      // new anchor.BN(merkleTreeIndex) // merkle tree index
      ()
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        systemProgram: SystemProgram.programId,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTree: merkle_tree_pubkey,
      })
      .remainingAccounts(leavesPdas)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ])
      .signers([signer])
      .rpc({
        commitment: "confirmed",
        preflightCommitment: "confirmed",
      });
  } catch (e) {
    console.log(" init Merkle tree update", e);
  }

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    MerkleTree: merkle_tree_pubkey,
    relayer: signer.publicKey,
    leavesPdas,
    current_instruction_index: 1,
    merkleTreeProgram,
  });

  await executeMerkleTreeUpdateTransactions({
    signer,
    merkleTreeProgram,
    merkle_tree_pubkey,
    provider,
    merkleTreeUpdateState,
    numberOfTransactions: 251,
  });

  await checkMerkleTreeUpdateStateCreated({
    connection: connection,
    merkleTreeUpdateState,
    MerkleTree: merkle_tree_pubkey,
    relayer: signer.publicKey,
    leavesPdas,
    current_instruction_index: 56,
    merkleTreeProgram,
  });

  // final tx to insert root
  let success = false;
  let recipient = Keypair.generate();
  try {
    let tx = await merkleTreeProgram.methods
      .insertRootMerkleTree(new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        // recipient: recipient.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey,
        logWrapper: SPL_NOOP_ADDRESS,
      })
      .remainingAccounts(leavesPdas).instruction();

    console.log(tx);

    await merkleTreeProgram.methods
      .insertRootMerkleTree(new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        // recipient: recipient.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey,
        logWrapper: SPL_NOOP_ADDRESS,
      })
      .remainingAccounts(leavesPdas)
      .signers([signer])
      .rpc({
        commitment: "confirmed",
        preflightCommitment: "confirmed",
      });
  } catch (e) {
    console.log(e);
  }

  await checkMerkleTreeBatchUpdateSuccess({
    connection: connection,
    merkleTreeUpdateState: merkleTreeUpdateState,
    merkleTreeAccountPrior,
    numberOfLeaves: leavesPdas.length * 2,
    leavesPdas,
    merkleTree: merkleTree,
    merkle_tree_pubkey: merkle_tree_pubkey,
    merkleTreeProgram,
  });
}

export async function executeMerkleTreeUpdateTransactions({
  merkleTreeProgram,
  merkleTreeUpdateState,
  merkle_tree_pubkey,
  provider,
  signer,
  numberOfTransactions,
}) {
  let arr = [];
  let i = 0;
  // console.log("Sending Merkle tree update transactions: ",numberOfTransactions)
  // the number of tx needs to increase with greater batchsize
  // 29 + 2 * leavesPdas.length is a first approximation
  for (let ix_id = 0; ix_id < numberOfTransactions; ix_id++) {
    const transaction = new Transaction();
    transaction.add(
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    );
    transaction.add(
      await merkleTreeProgram.methods
        .updateMerkleTree(new anchor.BN(i))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: merkle_tree_pubkey,
        })
        .instruction(),
    );
    i += 1;
    transaction.add(
      await merkleTreeProgram.methods
        .updateMerkleTree(new anchor.BN(i))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          merkleTree: merkle_tree_pubkey,
        })
        .instruction(),
    );
    i += 1;

    arr.push({ tx: transaction, signers: [signer] });
  }
  let error;
  await Promise.all(
    arr.map(async (tx, index) => {
      try {
        await provider.sendAndConfirm(tx.tx, tx.signers, confirmConfig);
      } catch (e) {
        console.log(e);
        error = e;
      }
    }),
  );
  return error;
}
