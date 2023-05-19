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
      description: "Solana public key of the Verifier",
    }),
  };

  async run() {
    const { args, flags } = await this.parse(VerifierCommand);
    const { method } = args;
    const { publicKey } = flags;

    const { loader, end } = getLoader(`Performing Verifier operation...`);

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
          this.error(`Failed to register the verifier: ${err}`);
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
          this.log("Verifier logged successfully!");
        } catch (err) {
          console.log(`Error while retrieving the verifier: ${verifierKey}`);
          this.error(`Failed to retrieve the verifier: ${err}`);
        }
      } else if (method === "list") {
        this.log("Listing Verifiers");

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
            this.log("No verifier accounts found");
          }
        } catch (err) {
          this.log("Error while listing the verifiers");
          this.error(`Failed to list the verifiers: ${err}`);
        }
      } else {
        this.error('Invalid command. Please use "set", "get", or "list"');
      }
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Failed to perform the Verifier operation: ${error}`);
    }
  }
}

export default VerifierCommand;

VerifierCommand.strict = false;
