import { Args, Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  setAnchorProvider,
} from "../../utils/utils";
import { getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import {
  ADMIN_AUTH_KEYPAIR,
  MINT,
  airdropSol,
  convertAndComputeDecimals,
} from "@lightprotocol/zk.js";
import { BN } from "@coral-xyz/anchor";

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
    `$ light airdrop --token SOL --amount 1.0 <userPublicKey>`,
    `$ light airdrop --token USDC --amount 10 <userPublicKey>`,
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
      const provider = await setAnchorProvider();

      if (token.toLowerCase() === "sol") {
        response = await airdropSol({
          provider: provider,
          amount: convertAndComputeDecimals(amount, new BN(1e9)),
          recipientPublicKey: new PublicKey(userPublicKey),
        });
      } else {
        let tokenAccount = await getOrCreateAssociatedTokenAccount(
          provider.connection,
          ADMIN_AUTH_KEYPAIR,
          MINT,
          new PublicKey(userPublicKey)
        );

        response = await mintTo(
          provider.connection,
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
      this.log(generateSolanaTransactionURL("tx", response!, "custom"));
      loader.stop();
    } catch (error) {
      loader.stop();
      this.error(`\nAirdrop failed: ${error}`);
    }
  }
}

AirdropCommand.strict = false;

export default AirdropCommand;
