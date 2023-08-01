import { Args, Command, Flags } from "@oclif/core";

import {
  POOL_TYPE,
  merkleTreeProgramId,
  IDL_MERKLE_TREE_PROGRAM,
} from "@lightprotocol/zk.js";

import {
  CustomLoader,
  getLightProvider,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";

class PoolCommand extends Command {
  static description = "Register a new pool type [default, SPL, SOL.";

  static examples = [
    "light pool default",
    "light pool SPL -p <pubKey>",
    "light pool SOL",
    "light pool list",
  ];

  static args = {
    method: Args.string({
      name: "method",
      description: "Method to perform: default, SPL, SOL, or list.",
      required: true,
    }),
  };

  static flags = {
    publicKey: Flags.string({
      char: "p",
      description: "Solana public key for the MINT.",
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  async run() {
    const { args, flags } = await this.parse(PoolCommand);
    const { method } = args;
    const { publicKey } = flags;

    const loader = new CustomLoader(
      method === "list"
        ? "Listing Pool Accounts..."
        : `Registering pool type...`
    );

    loader.start();

    const { connection } = await setAnchorProvider();

    let merkleTreeConfig = await getWalletConfig(connection);

    try {
      if (method === "default") {
        try {
          await merkleTreeConfig.registerPoolType(POOL_TYPE);
          this.log("\nSuccessfully registered the default pool type");
        } catch (error) {
          this.error("\nFailed to register the default pool type");
        }
      } else if (method === "SPL") {
        if (!publicKey) {
          this.error(
            "\nPlease provide the mint public key to register an SPL pool"
          );
        }

        const mintKey = new PublicKey(publicKey);

        try {
          await merkleTreeConfig.registerSplPool(POOL_TYPE, mintKey);
          this.log("\nSuccessfully registered the SPL pool");
        } catch (error) {
          this.error("\nFailed to register the SPL pool");
        }
      } else if (method === "SOL") {
        try {
          await merkleTreeConfig.registerSolPool(POOL_TYPE);
          this.log("\nSuccessfully registered the Sol pool");
        } catch (error) {
          this.error("\nFailed to register the Sol pool");
        }
      } else if (method === "list") {
        const provider = await getLightProvider();
        const merkleProgram = new Program(
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
            this.log("\nNo asset pool accounts found");
            this.log("\n");
          }
          if (poolAccounts.length > 0) {
            this.log("\nPool Accounts:");
            console.table(
              poolAccounts.map((account: any) => ({
                pubKey: `${account.publicKey}`,
              })),
              ["pubKey"]
            );
            this.log("\n");
          } else {
            this.log("\nNo pool accounts found");
          }
          this.log("\nSuccessfully listed the pools");
        } catch (error) {
          this.error("\nError while listing the pools");
        }
      } else {
        this.error(
          '\nInvalid method. Please use "default", "SPL", "SOL", or "list"'
        );
      }

      loader.stop();
    } catch (error) {
      loader.stop();

      this.error(`\nFailed to perform the pool operation: ${error}`);
    }
  }
}

export default PoolCommand;
