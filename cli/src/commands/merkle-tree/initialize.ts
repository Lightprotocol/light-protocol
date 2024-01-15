import { Command, Flags, ux } from "@oclif/core";
import { Keypair } from "@solana/web3.js";
import { getKeypairFromFile } from "@solana-developers/helpers";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";
import { MerkleTreeConfig } from "@lightprotocol/zk.js";

class InitializeCommand extends Command {
  static description = "Initialize new Merkle Trees.";

  static examples = ["light merkle-tree:initialize"];

  static flags = {
    "generate-mts-keypair": Flags.string({
      description: "Path to the MerkleTreeSet keypair to generate.",
      required: false,
    }),
    "use-mts-keypair": Flags.string({
      description: "Path to the MerkleTreeSet keypair to use.",
      required: false,
    }),
  };

  async run() {
    const loader = new CustomLoader("Initializing new Merkle Trees");
    loader.start();

    const { flags } = await this.parse(InitializeCommand);
    const generateMtsKeypair = flags["generate-mts-keypair"];
    const useMtsKeypair = flags["use-mts-keypair"];

    const anchorProvider = await setAnchorProvider();
    const merkleTreeConfig = await getWalletConfig(anchorProvider);

    if (generateMtsKeypair && useMtsKeypair) {
      this.error(
        "--generate-mts-keypair and --use-mts-keypair are mutually exclusive",
      );
    }

    let merkleTreeSet: Keypair;
    if (useMtsKeypair) {
      // If user provided another keypair, use it instead.
      merkleTreeSet = await getKeypairFromFile(useMtsKeypair);
    } else {
      // Generate a new keypair by default.
      merkleTreeSet = Keypair.generate();
    }

    const merkleTreeAuthorityAccountInfo =
      await merkleTreeConfig.getMerkleTreeAuthorityAccountInfo();

    const newMerkleTreeSetIndex =
      merkleTreeAuthorityAccountInfo.merkleTreeSetIndex;

    await merkleTreeConfig.initializeNewMerkleTreeSet(merkleTreeSet);
    this.log("Merkle Trees initialized successfully \x1b[32m✔\x1b[0m");
    ux.table(
      [
        {
          index: newMerkleTreeSetIndex.toString(),
          publicKey: merkleTreeSet.publicKey.toBase58(),
        },
      ],
      {
        index: {
          header: "Index",
        },
        publicKey: {
          header: "Public key",
        },
      },
    );
    loader.stop(false);
  }
}

export default InitializeCommand;
