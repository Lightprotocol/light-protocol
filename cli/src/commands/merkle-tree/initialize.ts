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
    const loader = new CustomLoader("Initializing new Merkle Trees");
    loader.start();

    const anchorProvider = await setAnchorProvider();
    let merkleTreeConfig = await getWalletConfig(anchorProvider);

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
    const newEventMerkleTree = MerkleTreeConfig.getEventMerkleTreePda(
      newEventMerkleTreeIndex
    );

    await merkleTreeConfig.initializeNewMerkleTrees();
    this.log("Merkle Trees initialized successfully \x1b[32m✔\x1b[0m");
    ux.table(
      [
        {
          type: "Transaction",
          index: newTransactionMerkleTreeIndex.toString(),
          publicKey: newTransactionMerkleTree.toBase58(),
        },
        {
          type: "Event",
          index: newEventMerkleTreeIndex.toString(),
          publicKey: newEventMerkleTree.toBase58(),
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
