import { Command, ux } from "@oclif/core";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";
import { MerkleTreeConfig } from "@lightprotocol/zk.js";

class InitializeCommand extends Command {
  static description = "Initialize new Merkle Trees.";

  static examples = ["light merkle-tree:initialize"];

  async run() {
    const loader = new CustomLoader("Initializing new Transaction Merkle Tree");
    loader.start();

    const { connection } = await setAnchorProvider();
    let merkleTreeConfig = await getWalletConfig(connection);

    let merkleTreeAuthorityAccountInfo =
      await merkleTreeConfig.getMerkleTreeAuthorityAccountInfo();

    const newTransactionMerkleTreeIndex =
      merkleTreeAuthorityAccountInfo.transactionMerkleTreeIndex;
    const newTransactionMerkleTree =
      MerkleTreeConfig.getTransactionMerkleTreePda(
        newTransactionMerkleTreeIndex
      );

    const newEventMerkleTreeIndex =
      merkleTreeAuthorityAccountInfo.eventMerkleTreeIndex;
    const newEventMerkleTree = MerkleTreeConfig.getTransactionMerkleTreePda(
      newEventMerkleTreeIndex
    );

    await merkleTreeConfig.initializeNewMerkleTrees();
    this.log(
      "Transaction Merkle Tree initialized successfully \x1b[32m✔\x1b[0m"
    );
    ux.table(
      [
        {
          type: "Transaction",
          index: newTransactionMerkleTreeIndex,
          publicKey: newTransactionMerkleTree,
        },
        {
          type: "Event",
          index: newEventMerkleTreeIndex,
          publicKey: newEventMerkleTree,
        },
      ],
      {
        type: {
          header: "Type",
        },
        index: {
          header: "Index",
        },
        publicKey: {
          header: "Public key",
        },
      }
    );
    loader.stop(false);
  }
}

export default InitializeCommand;
