import { Args, Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import {
  generateSolanaTransactionURL,
  getConnection,
  getLoader,
} from "../../utils";
import { getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { ADMIN_AUTH_KEYPAIR, MINT } from "light-sdk";

class AirdropCommand extends Command {
  static description = "Perform a native Solana or SPL airdrop to a user";

  static flags = {
    amount: Flags.integer({
      char: "a",
      description: "The amount to airdrop",
      required: true,
    }),
    token: Flags.string({
      char: "t",
      description: "The token to airdrop",
      required: true,
    }),
  };

  static examples = [
    `$ light airdrop --token SOL --amount 2000000000 <userPublicKey>`,
    `$ light airdrop --token USDC --amount 10000 <userPublicKey>`,
  ];

  static args = {
    userPublicKey: Args.string({
      name: "userPublicKey",
      description: "The Solana public key of the user",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(AirdropCommand);

    const { userPublicKey } = args;
    const { amount, token } = flags;

    const { loader, end } = getLoader("Performing the airdrop...");

    let response;

    try {
      const connection = await getConnection();

      if (token.toLowerCase() === "sol") {
        const res = await connection.requestAirdrop(
          new PublicKey(userPublicKey),
          amount
        );

        response = await connection.confirmTransaction(res, "confirmed");
      } else {
        let tokenAccount = await getOrCreateAssociatedTokenAccount(
          connection,
          ADMIN_AUTH_KEYPAIR,
          MINT,
          new PublicKey(userPublicKey)
        );

        response = await mintTo(
          connection,
          ADMIN_AUTH_KEYPAIR,
          MINT,
          tokenAccount.address,
          new PublicKey(userPublicKey),
          amount,
          []
        );
      }

      this.log(
        `Airdrop successful for user: ${userPublicKey}, amount: ${amount}`
      );
      this.log(
        generateSolanaTransactionURL("tx", response.toString(), "custom")
      );
      end(loader);
    } catch (error) {
      end(loader);
      this.error(`Airdrop failed: ${error}`);
    }
  }
}

AirdropCommand.strict = false;

export default AirdropCommand;
