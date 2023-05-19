import { Args, Command, Flags } from "@oclif/core";
import { merkleTreeProgramId, IDL_MERKLE_TREE_PROGRAM } from "light-sdk";

import { getLoader, getWalletConfig, setAnchorProvider } from "../../utils";

import { PublicKey } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";

class VerifierCommand extends Command {
  static description = "Register a new verifier for a Merkle Tree";

  static examples = [
    "light verifier set -p <pubKey>",
    "light verifier get -p <pubKey>",
    "light verifier list",
  ];

  static args = {
    method: Args.string({
      name: "method",
      description: "Method to perform: set, get, or list",
      required: true,
    }),
  };

  static flags = {
    publicKey: Flags.string({
      char: "p",
      description: "Public key of the Verifier",
    }),
  };

  async run() {
    const { args, flags } = await this.parse(VerifierCommand);
    const { method } = args;
    const { publicKey } = flags;

    const { loader, end } = getLoader(`Registering Verifier...`);

    const { connection } = await setAnchorProvider();
    const merkleTreeConfig = await getWalletConfig(connection);

    try {
      if (method === "set") {
        if (!publicKey) {
          this.error("Please provide the public key of the verifier");
        }

        const verifierKey = new PublicKey(publicKey);

        try {
          await merkleTreeConfig.registerVerifier(verifierKey);
          this.log("Verifier registered successfully!");
        } catch (err) {
          this.error(`${err}`);
        }
      } else if (method === "get") {
        if (!publicKey) {
          this.error("Please provide the public key of the verifier");
        }

        const verifierKey = new PublicKey(publicKey);

        try {
          const verifierPdaAccount =
            await merkleTreeConfig.getRegisteredVerifierPda(verifierKey);
          console.log(verifierPdaAccount);
          this.log("Verifier Successfully Logged");
        } catch (err) {
          console.log(`Error while registering verifier ${verifierKey}`);
          this.error(`${err}`);
        }
      } else if (method === "list") {
        this.log("Listing Verifier");

        const merkleProgram = new Program(
          IDL_MERKLE_TREE_PROGRAM,
          merkleTreeProgramId
        );

        try {
          const verifierAccounts =
            await merkleProgram.account.registeredVerifier.all();

          if (verifierAccounts.length > 0) {
            this.log("\nVerifier Accounts:");
            console.table(
              verifierAccounts.map((account: any) => {
                return { pubKey: `${account.publicKey}` };
              }),
              ["pubKey"]
            );
          } else {
            this.log("No verifier account found");
          }
        } catch (err) {
          this.log("Error while listing verifiers");
          this.error(`${err}`);
        }
      } else {
        this.error('Invalid command. Please use "set", "get", or "list"');
      }
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Command Failed: ${error}`);
    }
  }
}

export default VerifierCommand;

VerifierCommand.strict = false;
