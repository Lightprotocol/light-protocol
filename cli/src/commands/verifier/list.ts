import { Command, ux } from "@oclif/core";
import { Program } from "@coral-xyz/anchor";
import {
  IDL_MERKLE_TREE_PROGRAM,
  merkleTreeProgramId,
} from "@lightprotocol/zk.js";
import { setAnchorProvider } from "../../utils";

class VerifierListCommand extends Command {
  static description = "List registered verifiers.";

  static examples = ["light merkle-tree-authority:verifier-list"];

  async run() {
    await setAnchorProvider();

    const merkleTreeProgram = new Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId
    );
    const verifierAccounts =
      await merkleTreeProgram.account.registeredVerifier.all();
    ux.table(verifierAccounts, {
      publicKey: {
        header: "Public key",
        get: (account) => account.account.pubkey,
      },
    });
  }
}

export default VerifierListCommand;
