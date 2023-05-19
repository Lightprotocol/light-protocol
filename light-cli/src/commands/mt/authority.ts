import { Args, Command, Flags } from "@oclif/core";
import { MERKLE_TREE_AUTHORITY_PDA } from "light-sdk";
import { getLoader, getWalletConfig, setAnchorProvider } from "../../utils";

import { PublicKey } from "@solana/web3.js";

class AuthorityCommand extends Command {
  static description = "Initialize, set, or get the Merkle Tree Authority";

  static examples = [
    "light authority init",
    "light authority set -p <publicKey>",
    "light authority get",
  ];

  static flags = {
    publicKey: Flags.string({
      char: "p",
      description: "Solana Public key of the authority",
    }),
  };

  static args = {
    method: Args.string({
      name: "method",
      description: "Method to perform: init, set, or get",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(AuthorityCommand);
    const { method } = args;
    const { publicKey } = flags;

    const { loader, end } = getLoader(
      `${
        method === "get"
          ? "Retrieving"
          : method === "init"
          ? "Initializing"
          : "Setting"
      } the authority...\n`
    );

    try {
      const { connection } = await setAnchorProvider();

      let merkleTreeConfig = await getWalletConfig(connection);

      if (method === "init") {
        try {
          await merkleTreeConfig.initMerkleTreeAuthority();
          this.log("Merkle Tree Authority initialized successfully");
        } catch (error) {
          this.error(`Failed to initialize Merkle Tree Authority: ${error}`);
        }
      } else if (method === "set") {
        if (!publicKey) {
          this.error(
            "Please provide the public key of the new authority account"
          );
        }
        try {
          await merkleTreeConfig.updateMerkleTreeAuthority(
            new PublicKey(publicKey),
            true
          );
          this.log(`Authority account updated: ${new PublicKey(publicKey)}`);
        } catch (error) {
          this.error(`Failed to update authority account: ${error}`);
        }
      } else if (method === "get") {
        try {
          const authority =
            await merkleTreeConfig.merkleTreeProgram.account.merkleTreeAuthority.fetch(
              MERKLE_TREE_AUTHORITY_PDA
            );
          this.log("Authority Account:", authority);
        } catch (error) {
          this.error(`Failed to retrieve authority account: ${error}`);
        }
      } else {
        this.error("Invalid command. Please use 'init', 'set', or 'get'");
      }
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Failed to perform authority operation: ${error}`);
    }
  }
}

AuthorityCommand.strict = false;

export default AuthorityCommand;
