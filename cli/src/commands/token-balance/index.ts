import { Command, Flags } from "@oclif/core";
import { CustomLoader, rpc } from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddressInterface } from "@lightprotocol/compressed-token";

class TokenBalanceCommand extends Command {
  static summary = "Get token balance (light token account + compressed)";

  static examples = [
    "$ light token-balance --mint=<ADDRESS> --owner=<ADDRESS>",
  ];

  static flags = {
    owner: Flags.string({
      description: "Address of the token owner.",
      required: true,
    }),
    mint: Flags.string({
      description: "Mint address of the token account.",
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

      // Fetch light token account (hot) balance
      let hotBalance = BigInt(0);
      const ataAddress = getAssociatedTokenAddressInterface(refMint, refOwner);
      try {
        const ataBalance = await rpc().getTokenAccountBalance(ataAddress);
        hotBalance = BigInt(ataBalance.value.amount);
      } catch {
        // Token account may not exist; treat as zero.
      }

      // Fetch compressed (cold) balance
      let coldBalance = BigInt(0);
      const tokenAccounts = await rpc().getCompressedTokenAccountsByOwner(
        refOwner,
        { mint: refMint },
      );

      tokenAccounts.items.forEach((account: any) => {
        coldBalance += BigInt(account.parsed.amount.toString());
      });

      loader.stop(false);

      const totalBalance = hotBalance + coldBalance;

      console.log(
        `\x1b[1mLight token account balance:\x1b[0m ${hotBalance.toString()}`,
      );
      console.log(
        `\x1b[1mCompressed light token balance:\x1b[0m ${coldBalance.toString()}`,
      );
      console.log(
        `\x1b[1mTotal balance:\x1b[0m ${totalBalance.toString()}`,
      );
    } catch (error) {
      this.error(`Failed to get balance!\n${error}`);
    }
  }
}

export default TokenBalanceCommand;
