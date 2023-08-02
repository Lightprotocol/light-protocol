import { Args, Command, Flags } from "@oclif/core";
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  MINT,
  convertAndComputeDecimals,
  airdropSplToAssociatedTokenAccount,
  airdropSol,
} from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  setAnchorProvider,
} from "../../utils/utils";

class AirdropCommand extends Command {
  static description = "Perform a native SOL or SPL airdrop to a user.";
  static examples = [
    `$ light airdrop 1.5 <RECIPIENT_ADDRESS>`,
    `$ light airdrop --token USDC 15 <RECIPIENT_ADDRESS> -v`,
  ];

  static flags = {
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol.",
      default: "SOL",
      parse: async (token: string) => token.toUpperCase(),
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The airdrop amount to request.",
      required: true,
    }),
    recipient_address: Args.string({
      name: "RECIPIENT_ADDRESS",
      description: "The account address of recipient.",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(AirdropCommand);
    const amount = args.amount;
    const recipient_address = new PublicKey(args.recipient_address);
    const { token } = flags;

    const loader = new CustomLoader(
      `Requesting airdrop of ${amount} ${token}...`
    );
    loader.start();

    let transactionSignature: string;

    try {
      const provider = await setAnchorProvider();

      if (token.toLowerCase() === "SOL") {
        transactionSignature = await airdropSol({
          connection: provider.connection,
          recipientPublicKey: recipient_address,
          lamports: convertAndComputeDecimals(amount, new BN(1e9)).toNumber(),
        });
      } else {
        transactionSignature = await airdropSplToAssociatedTokenAccount(
          provider.connection,
          parseInt(amount) * 100,
          recipient_address
        );
      }

      this.log(`\n\x1b[1mRecipient:\x1b[0m ${recipient_address}`);
      this.log(`\x1b[1mSignature:\x1b[0m ${transactionSignature}`);
      if (token.toLowerCase() !== "SOL")
        this.log(`\x1b[1mMint:\x1b[0m      ${MINT}`);
      this.log(
        generateSolanaTransactionURL("tx", transactionSignature!, "custom")
      );
      this.log("\nAirdrop Successful \x1b[32m✔\x1b[0m");

      loader.stop(false);
    } catch (error) {
      this.error(`Failed to airdrop ${token}!\n${error}`);
    }
  }
}

export default AirdropCommand;
