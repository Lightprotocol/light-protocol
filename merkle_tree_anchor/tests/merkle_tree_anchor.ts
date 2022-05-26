import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { MerkleTreeAnchor } from "../target/types/merkle_tree_anchor";

describe("merkle_tree_anchor", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.MerkleTreeAnchor as Program<MerkleTreeAnchor>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
