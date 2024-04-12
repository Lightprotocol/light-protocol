import { Command, Flags } from "@oclif/core";
import { CustomLoader, getSolanaRpcUrl } from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";

class BalanceCommand extends Command {
  static summary = "Get balance";

  static examples = ["$ light balance --mint=<ADDRESS> --owner=<ADDRESS>"];

  static flags = {
    owner: Flags.string({
      description: "Address of the compressed token owner.",
      required: true,
    }),
    mint: Flags.string({
      description: "Mint address of the compressed token account.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(BalanceCommand);
    const loader = new CustomLoader(`Performing balance...\n`);
    loader.start();
    try {
      const refMint = new PublicKey(flags["mint"]);
      const refOwner = new PublicKey(flags["owner"]);
      const rpc = createRpc(getSolanaRpcUrl());
      const tokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        refOwner,
        { mint: refMint },
      );

      loader.stop(false);

      if (tokenAccounts.length === 0) {
        console.log("No token accounts found");
        return;
      }

      const compressedTokenAccount = tokenAccounts.find((acc) =>
        acc.parsed.mint.equals(refMint),
      );
      if (compressedTokenAccount === undefined) {
        console.log("No token accounts found");
        return;
      }
      console.log(
        "\x1b[1mBalance:\x1b[0m ",
        compressedTokenAccount.parsed.amount.toString(),
      );
      console.log("balance successful");
    } catch (error) {
      this.error(`Failed to get balance!\n${error}`);
    }
  }
}

export default BalanceCommand;
