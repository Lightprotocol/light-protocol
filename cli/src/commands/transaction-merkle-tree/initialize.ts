import { Args, Command } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";

class TransactionInitializeCommand extends Command {
  static description = "Initialize a new Transaction Merkle Tree.";

  static examples = ["light transaction-merkle-tree:initialize -p <pubKey>"];

  static args = {
    oldTree: Args.string({
      description: "Solana public key of the old Merkle Tree.",
      required: true,
    }),
    newTree: Args.string({
      name: "new_tree",
      description: "Solana public key of the new Merkle Tree.",
      required: true,
    }),
  };

  async run() {
    const { args } = await this.parse(TransactionInitializeCommand);
    const { oldTree, newTree } = args;

    const oldMerkleTreePubkey = new PublicKey(oldTree);
    const newMerkleTreePubkey = new PublicKey(newTree);

    const loader = new CustomLoader("Initializing new Transaction Merkle Tree");
    loader.start();

    const { connection } = await setAnchorProvider();

    const newMerkleTreeAccountInfo = await connection.getAccountInfo(
      newMerkleTreePubkey
    );

    if (newMerkleTreeAccountInfo) {
      this.log("Transaction Merkle Tree already initialized");
    } else {
      let merkleTreeConfig = await getWalletConfig(connection);
      await merkleTreeConfig.initializeNewTransactionMerkleTree(
        oldMerkleTreePubkey,
        newMerkleTreePubkey
      );
      this.log(
        "Transaction Merkle Tree initialized successfully \x1b[32m✔\x1b[0m"
      );
    }
    loader.stop(false);
  }
}

export default TransactionInitializeCommand;
