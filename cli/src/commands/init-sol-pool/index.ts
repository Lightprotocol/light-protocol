import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import {  createRpc, initSolOmnibusAccount } from "@lightprotocol/stateless.js";
import { getKeypairFromFile } from "@solana-developers/helpers";

class InitSolPoolCommand extends Command {
  static summary = "Initialize the system sol-pool pda.";

  static examples = ["$ light init-sol-pool --authority pathToFile"];

  static flags = {
    to: Flags.string({
      description: "Specify the pool authority keypair file path. defaults to your default local solana wallet file path.",
      required: false,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(InitSolPoolCommand);
    const payer = defaultSolanaWalletKeypair();
    let authorityKeypair = payer;

    if (flags["authority"]) {
       const authorityPath = flags["authority"];
        authorityKeypair = await getKeypairFromFile(authorityPath);

    }

    const loader = new CustomLoader(`Performing init-sol-pool...\n`);
    loader.start();

    try {

      const rpc = createRpc(getSolanaRpcUrl());
      const txId = await initSolOmnibusAccount(
        rpc,
        payer,
        authorityKeypair,
      );
      loader.stop(false);
      console.log(
        "\x1b[32minit-sol-pool:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("init-sol-pool successful");
    } catch (error: any) {
      if (error.toString().includes("0x0")) {
        this.error("Failed to init-sol-pool!\nAlready inited.");
      } else {
        this.error(`Failed to init-sol-pool!\n${error}.`);
      }
    }
  }
}

export default InitSolPoolCommand;
