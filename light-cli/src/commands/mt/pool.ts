import { Args, Command, Flags } from "@oclif/core";

import {
  ADMIN_AUTH_KEYPAIR,
  POOL_TYPE,
  merkleTreeProgramId,
  IDL_MERKLE_TREE_PROGRAM,
} from "light-sdk";

import * as anchor from "@coral-xyz/anchor";
import {
  getLightProvider,
  getLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils";
import { PublicKey } from "@solana/web3.js";

class PoolCommand extends Command {
  static description = "Register a new pool type [default, spl, sol]";

  static examples = [
    "light pool default",
    "light pool spl -p <pubKey>",
    "light pool sol",
    "light pool list",
  ];

  static args = {
    method: Args.string({
      name: "method",
      description: "Method to perform: default, spl, sol, or list",
      required: true,
    }),
  };

  static flags = {
    publicKey: Flags.string({
      char: "p",
      description: "Solana public key for the MINT",
    }),
  };

  async run() {
    const { args, flags } = await this.parse(PoolCommand);
    const { method } = args;
    const { publicKey } = flags;

    const { loader, end } = getLoader(
      method === "list"
        ? "Listing Pool Accounts..."
        : `Registering pool type...`
    );

    const { connection } = await setAnchorProvider();

    let merkleTreeConfig = await getWalletConfig(connection);

    try {
      if (method === "default") {
        try {
          await merkleTreeConfig.registerPoolType(POOL_TYPE);
          this.log("Successfully registered the default pool type");
        } catch (error) {
          this.error("Failed to register the default pool type");
        }
      } else if (method === "spl") {
        if (!publicKey) {
          this.error(
            "Please provide the mint public key to register an SPL pool"
          );
        }

        const mintKey = new PublicKey(publicKey);

        try {
          await merkleTreeConfig.registerSplPool(POOL_TYPE, mintKey);
          this.log("Successfully registered the SPL pool");
        } catch (error) {
          this.error("Failed to register the SPL pool");
        }
      } else if (method === "sol") {
        try {
          await merkleTreeConfig.registerSolPool(POOL_TYPE);
          this.log("Successfully registered the Sol pool");
        } catch (error) {
          this.error("Failed to register the Sol pool");
        }
      } else if (method === "list") {
        const provider = await getLightProvider(ADMIN_AUTH_KEYPAIR);
        const merkleProgram = new anchor.Program(
          IDL_MERKLE_TREE_PROGRAM,
          merkleTreeProgramId,
          provider.provider!
        );

        try {
          const assetPoolsAccounts =
            await merkleProgram.account.registeredAssetPool.all();
          const poolAccounts =
            await merkleProgram.account.registeredPoolType.all();

          if (assetPoolsAccounts.length > 0) {
            this.log("\nAsset Pool Accounts:");
            console.table(
              assetPoolsAccounts.map((account: any) => ({
                pubKey: `${account.publicKey}`,
              })),
              ["pubKey"]
            );
          } else {
            this.log("No asset pool accounts found");
            this.log("\n");
          }
          if (poolAccounts.length > 0) {
            this.log("Pool Accounts:");
            console.table(
              poolAccounts.map((account: any) => ({
                pubKey: `${account.publicKey}`,
              })),
              ["pubKey"]
            );
            this.log("\n");
          } else {
            this.log("No pool accounts found");
          }
          this.log("Successfully listed the pools");
        } catch (error) {
          this.error("Error while listing the pools");
        }
      } else {
        this.error(
          'Invalid method. Please use "default", "spl", "sol", or "list"'
        );
      }

      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Failed to perform the pool operation: ${error}`);
    }
  }
}

export default PoolCommand;
