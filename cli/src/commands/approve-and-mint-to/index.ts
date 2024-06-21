import { Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
  rpc,
} from "../../utils/utils";
import { getKeypairFromFile } from "@solana-developers/helpers";
import { Keypair, PublicKey } from "@solana/web3.js";
import { approveAndMintTo } from "@lightprotocol/compressed-token";
import { getTestRpc } from "@lightprotocol/stateless.js";
import { WasmFactory } from "@lightprotocol/hasher.rs";

class ApproveAndMintToCommand extends Command {
  static summary =
    "Mint tokens to a compressed account via external mint authority";

  static examples = [
    "$ light approve-and-mint-to --mint PublicKey --to PublicKey --amount 1000",
  ];

  static flags = {
    "mint-authority": Flags.string({
      description:
        "Specify the filepath of the mint authority keypair. Defaults to your local solana wallet.",
      required: false,
    }),
    mint: Flags.string({
      description: "Specify the mint address.",
      required: true,
    }),
    to: Flags.string({
      description: "Specify the recipient address.",
      required: true,
    }),
    amount: Flags.integer({
      description: "Amount to mint, in tokens.",
      required: true,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(ApproveAndMintToCommand);
    const mint = flags["mint"];
    const to = flags["to"];
    const amount = flags["amount"];
    if (!mint || !to || !amount) {
      throw new Error("Invalid arguments");
    }

    const loader = new CustomLoader(`Performing approve-and-mint-to...\n`);
    loader.start();

    try {
      const mintPublicKey = new PublicKey(mint);
      const toPublicKey = new PublicKey(to);
      const payer = defaultSolanaWalletKeypair();

      let mintAuthority: Keypair = payer;
      if (flags["mint-authority"] !== undefined) {
        mintAuthority = await getKeypairFromFile(flags["mint-authority"]);
      }
      const txId = await approveAndMintTo(
        rpc(),
        payer,
        mintPublicKey,
        toPublicKey,
        mintAuthority,
        amount,
      );
      loader.stop(false);
      console.log(
        "\x1b[1mMint tx:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("approve-and-mint-to successful");
    } catch (error) {
      this.error(`Failed to approve-and-mint-to!\n${error}`);
    }
  }
}

export default ApproveAndMintToCommand;
