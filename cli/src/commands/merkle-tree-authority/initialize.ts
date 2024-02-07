import { promises as fs } from "fs";
import { Command, Flags } from "@oclif/core";
import { Keypair } from "@solana/web3.js";
import { getKeypairFromFile } from "@solana-developers/helpers";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";
import { CONFIG_PATH } from "../../psp-utils";
import { MerkleTreeConfig } from "@lightprotocol/zk.js";
class InitializeCommand extends Command {
  static description = "Initialize the Merkle Tree Authority.";

  static examples = ["light merkle-tree-authority:initialize"];

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
    const loader = new CustomLoader("Initializing Merkle Tree Authority");
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

    const accountInfo = await anchorProvider.connection.getAccountInfo(
      MerkleTreeConfig.getMerkleTreeAuthorityPda(),
    );
    if (accountInfo && accountInfo.data.length > 0) {
      this.error("Merkle Tree Authority already initialized");
    } else {
      await merkleTreeConfig.initMerkleTreeAuthority({
        merkleTreeSet,
      });
      this.log(
        "Merkle Tree Authority initialized successfully \x1b[32m✔\x1b[0m",
      );
    }

    this.log("MerkleTreeSet public key:", merkleTreeSet.publicKey.toBase58());

    if (!useMtsKeypair) {
      // If the generated keypair was used, save it to a file.
      const secretKeyJson = JSON.stringify(Array.from(merkleTreeSet.secretKey));
      if (generateMtsKeypair) {
        // Save it to file provided by the user.
        await fs.writeFile(generateMtsKeypair, secretKeyJson, "utf8");
        this.log("MerkleTreeSet secret key written in:", generateMtsKeypair);
      } else {
        // Save it to the default path.
        const keypairPath =
          process.env.HOME + CONFIG_PATH + "merkle-tree-set-0.json";
        await fs.writeFile(keypairPath, secretKeyJson, "utf8");
        this.log("MerkleTreeSet secret key written in:", keypairPath);
      }
    }

    loader.stop(false);
  }
}

export default InitializeCommand;
