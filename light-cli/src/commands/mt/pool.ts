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
      description: "Method to perform: default, spl or sol",
      required: true,
    }),
  };

  static flags = {
    publicKey: Flags.string({
      char: "p",
      description: "Public key for the MINT",
    }),
  };

  async run() {
    const { args, flags } = await this.parse(PoolCommand);
    const { method } = args;
    const { publicKey } = flags;

    const { loader, end } = getLoader(
      method === "list" ? "Listing Pool Accounts..." : `Registering pool...`
    );

    const { connection } = await setAnchorProvider();

    let merkleTreeConfig = await getWalletConfig(connection);

    try {
      if (method === "default") {
        this.log("Registering pool type...");
        try {
          await merkleTreeConfig.registerPoolType(POOL_TYPE);
          this.log("Successfully registered pool type");
        } catch (error) {
          this.error("Failed to register pool type");
        }
      } else if (method === "spl") {
        if (!publicKey) {
          this.error("Mint pubKey required for register Spl Pool");
        }

        const mintKey = new PublicKey(publicKey);

        try {
          await merkleTreeConfig.registerSplPool(POOL_TYPE, mintKey);
          this.log("Successfully registered spl pool");
        } catch (error) {
          this.error("Failed to register spl pool");
        }
      } else if (method === "sol") {
        try {
          await merkleTreeConfig.registerSolPool(POOL_TYPE);
          this.log("Successfully registered sol pool");
        } catch (error) {
          this.error("Failed to register sol pool");
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
            this.log("No asset");
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
            this.log("No pool account found");
          }
          this.log("Pools Successfully Listed");
        } catch (error) {
          this.error("Error while listing verifiers");
        }
      } else {
        this.error(
          'Invalid method. Please use "pooltype", "spl", "sol", or "list"'
        );
      }

      end(loader);
    } catch (error) {
      let errorMessage = "Aborted.";
      if (error instanceof Error) {
        errorMessage = error.message;
      }
      end(loader);
      this.error(errorMessage);
    }
  }
}

export default PoolCommand;
