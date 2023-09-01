import { Command, ux } from "@oclif/core";
import { getWalletConfig, setAnchorProvider } from "../../utils/utils";

class GetCommand extends Command {
  static description = "Get the Merkle Tree Authority";

  static examples = ["light merkle-tree-authority:get"];

  async run() {
    const { connection } = await setAnchorProvider();
    let merkleTreeConfig = await getWalletConfig(connection);

    if (!(await merkleTreeConfig.isMerkleTreeAuthorityInitialized())) {
      this.logToStderr("Merkle Tree Authority is not initialized");
      this.exit(1);
    }

    const merkleTreeAuthorityAccountInfo =
      await merkleTreeConfig.getMerkleTreeAuthorityAccountInfo();
    ux.table(
      [
        {
          transactionMerkleTreeIndex:
            merkleTreeAuthorityAccountInfo.transactionMerkleTreeIndex.toString(),
          eventMerkleTreeIndex:
            merkleTreeAuthorityAccountInfo.eventMerkleTreeIndex.toString(),
          registeredAssetIndex:
            merkleTreeAuthorityAccountInfo.registeredAssetIndex.toString(),
          enableNfts: merkleTreeAuthorityAccountInfo.enableNfts,
          enablePermissionlessSplTokens:
            merkleTreeAuthorityAccountInfo.enablePermissionlessSplTokens,
          enablePermissionlessMerkleTreeRegistration:
            merkleTreeAuthorityAccountInfo.enablePermissionlessMerkleTreeRegistration,
        },
      ],
      {
        transactionMerkleTreeIndex: {
          header: "Transaction Merkle Tree index",
        },
        eventMerkleTreeIndex: {
          header: "Event Merkle Tree index",
        },
        registeredAssetIndex: {
          header: "Registered asset index",
        },
        enableNfts: {
          header: "Enable NFTs",
        },
        enablePermissionlessSplTokens: {
          header: "Enable permissionless SPL tokens",
        },
        enablePermissionlessMerkleTreeRegistration: {
          header: "Enable permissionless Merkle Tree registration",
        },
      }
    );
  }
}

export default GetCommand;
