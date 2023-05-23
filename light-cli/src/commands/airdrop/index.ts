import { Args, Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  getConnection,
} from "../../utils/utils";
import { getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { ADMIN_AUTH_KEYPAIR, MINT } from "@lightprotocol/zk.js";

class AirdropCommand extends Command {
  static description = "Perform a native Solana or SPL airdrop to a user";

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

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
      description: "The solana public key of the user",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(AirdropCommand);

    const { userPublicKey } = args;
    const { amount, token } = flags;

    const loader = new CustomLoader("Performing the airdrop...");
    loader.start();

    let response;

    try {
      const connection = await getConnection();

      if (token.toLowerCase() === "sol") {
        console.log("here -==========>",connection.rpcEndpoint);

        response = await connection.requestAirdrop(
          new PublicKey(userPublicKey),
          amount
        );

        await connection.confirmTransaction(response, "confirmed");
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
        `\nAirdrop successful for user: ${userPublicKey}, amount: ${amount} ${token}`
      );
      this.log(
        generateSolanaTransactionURL("tx", response.toString(), "custom")
      );
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`\nAirdrop failed: ${error}`);
    }
  }
}

AirdropCommand.strict = false;

export default AirdropCommand;
