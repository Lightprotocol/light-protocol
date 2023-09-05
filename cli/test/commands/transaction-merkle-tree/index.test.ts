import { expect, test } from "@oclif/test";
import { initTestEnv } from "../../../src/utils/initTestEnv";
import { BN_0, BN_1, BN_2, MerkleTreeConfig } from "@lightprotocol/zk.js";
import { BN } from "@coral-xyz/anchor";

describe("merkle-tree", () => {
  before(async () => {
    await initTestEnv({ skip_system_accounts: true });
  });
  // TODO(vadorovsky): Teach `initTestEnv` to initialize only some accounts,
  // then we will be able to not initialize the Merkle Tree Authority this
  // way.
  test
    .stdout({ print: true })
    .command(["merkle-tree-authority:initialize"])
    .it("Initialize Merkle Tree Authority", ({ stdout }) => {
      expect(stdout).to.contain(
        "Merkle Tree Authority initialized successfully"
      );
    });
  test
    .stdout({ print: true })
    .command([
      "transaction-merkle-tree:initialize",
      MerkleTreeConfig.getTransactionMerkleTreePda(BN_0).toBase58(),
      MerkleTreeConfig.getTransactionMerkleTreePda(BN_1).toBase58(),
    ])
    .it("Initialize Transaction Merkle Tree (index 1)", ({ stdout }) => {
      expect(stdout).to.contain(
        "Transaction Merkle Tree initialized successfully"
      );
    });
  test
    .stdout()
    .command([
      "transaction-merkle-tree:initialize",
      MerkleTreeConfig.getTransactionMerkleTreePda(BN_1).toBase58(),
      MerkleTreeConfig.getTransactionMerkleTreePda(BN_2).toBase58(),
    ])
    .it("Initialize Transaction Merkle Tree (index 2)", ({ stdout }) => {
      expect(stdout).to.contain(
        "Transaction Merkle Tree initialized successfully"
      );
    });
});
