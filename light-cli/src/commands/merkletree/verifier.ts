import { Args, Command, Flags } from "@oclif/core";
import {
  ADMIN_AUTH_KEYPAIR,
  MERKLE_TREE_KEY,
  MerkleTreeProgram,
  merkleTreeProgramId,
} from "light-sdk";

import * as anchor from "@coral-xyz/anchor";

import {
  getLocalProvider,
  getWalletConfig,
  readPayerFromIdJson,
} from "../../utils";

import { PublicKey } from "@solana/web3.js";

class VerifierCommand extends Command {
  static description = "Register a new verifier for a Merkle Tree";

  static examples = [
    "light-cli verifier set -p <pubKey>",
    "light-cli verifier get -p <pubKey>",
    "light-cli verifier list",
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

    try {
      if (method === "set") {
        if (!publicKey) {
          this.error("Please provide the public key of the verifier");
          return;
        }

        const verifierKey = new PublicKey(publicKey);

        this.log("Registering Verifiers...");

        const payer = new anchor.Wallet(readPayerFromIdJson());
        const provider = await getLocalProvider(payer);
        let merkleTreeConfig = await getWalletConfig(
          provider,
          MERKLE_TREE_KEY,
          readPayerFromIdJson()
        );

        try {
          await merkleTreeConfig.registerVerifier(verifierKey);
          this.log("Verifiers registered successfully!");
          this.log(`Verifier PubKey: ${verifierKey}\n`);
        } catch (err) {
          this.error(err.message);
        }
      } else if (method === "get") {
        if (!publicKey) {
          this.error("Please provide the public key of the verifier");
        }

        const verifierKey = new PublicKey(publicKey);

        this.log("Getting Verifier");

        const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
        const provider = await getLocalProvider(payer);
        let merkleTreeConfig = await getWalletConfig(provider);

        try {
          const verifierPdaAccount =
            await merkleTreeConfig.getRegisteredVerifierPda(verifierKey);
          console.log(verifierPdaAccount);
          this.log("Verifier Successfully Logged");
        } catch (err) {
          console.log(`Error while registering verifier ${verifierKey}`);
          this.error(err.message);
        }
      } else if (method === "list") {
        this.log("Listing Verifier");

        const payer = new anchor.Wallet(ADMIN_AUTH_KEYPAIR);
        const provider = await getLocalProvider(payer);
        const merkleProgram = new MerkleTreeProgram(provider);

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

            this.log("Verifiers Successfully Listed");
          }
        } catch (err) {
          this.log("Error while listing verifiers");
          this.error(err.message);
        }
      } else {
        this.error('Invalid command. Please use "set", "get", or "list"');
      }
    } catch (error) {
      this.error("Command Failed");
    }
  }
}

export default VerifierCommand;

VerifierCommand.strict = false;
