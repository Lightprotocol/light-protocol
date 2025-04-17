import { Command, Flags } from "@oclif/core";
import { CustomLoader, rpc } from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";

class TokenBalanceCommand extends Command {
  static summary = "Get balance";

  static examples = [
    "$ light token-balance --mint=<ADDRESS> --owner=<ADDRESS>",
  ];

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
    const { flags } = await this.parse(TokenBalanceCommand);
    const loader = new CustomLoader(`Performing balance...\n`);
    loader.start();
    try {
      const refMint = new PublicKey(flags["mint"]);
      const refOwner = new PublicKey(flags["owner"]);

      const tokenAccounts = await rpc().getCompressedTokenAccountsByOwner(
        refOwner,
        { mint: refMint },
      );

      loader.stop(false);

      // Handle case when no token accounts are found
      if (tokenAccounts.items.length === 0) {
        console.log("\x1b[1mBalance:\x1b[0m 0");
        console.log("No token accounts found");
        return;
      }

      const compressedTokenAccounts = tokenAccounts.items.filter((acc) =>
        acc.parsed.mint.equals(refMint),
      );

      if (compressedTokenAccounts.length === 0) {
        console.log("\x1b[1mBalance:\x1b[0m 0");
        console.log("No token accounts found for this mint");
        return;
      }

      let totalBalance = BigInt(0);

      compressedTokenAccounts.forEach((account) => {
        const amount = account.parsed.amount;
        totalBalance += BigInt(amount.toString());
      });

      console.log(`\x1b[1mBalance:\x1b[0m ${totalBalance.toString()}`);
    } catch (error) {
      this.error(`Failed to get balance!\n${error}`);
    }
  }
}

export default TokenBalanceCommand;
