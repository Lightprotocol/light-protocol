import { Command, Flags } from "@oclif/core";
import { getKeypairFromFile } from "@solana-developers/helpers";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { createMint } from "@lightprotocol/compressed-token";
import { Keypair } from "@solana/web3.js";
import { createRpc } from "@lightprotocol/stateless.js";

const DEFAULT_DECIMAL_COUNT = 9;

class CreateMintCommand extends Command {
  static summary = "Create a new compressed token mint";

  static examples = ["$ light create-mint --mint-decimals 5"];

  static flags = {
    "mint-keypair": Flags.string({
      description:
        "Provide a path to a mint keypair file. Defaults to a random keypair",
      required: false,
    }),
    "mint-authority": Flags.string({
      description:
        "Specify a path to the mint authority keypair file. Defaults to your default local solana wallet file path",
      required: false,
    }),
    "mint-decimals": Flags.integer({
      description: `Number of base 10 digits to the right of the decimal place [default: ${DEFAULT_DECIMAL_COUNT}]`,
      required: false,
      default: DEFAULT_DECIMAL_COUNT,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(CreateMintCommand);

    const loader = new CustomLoader(`Performing create-mint...\n`);
    loader.start();
    try {
      const payer = defaultSolanaWalletKeypair();
      console.log(payer.publicKey.toBase58());
      const mintDecimals = this.getMintDecimals(flags);
      const mintKeypair = await this.getMintKeypair(flags);
      const mintAuthority = await this.getMintAuthority(flags, payer);
      const rpc = createRpc(getSolanaRpcUrl());
      const { mint, transactionSignature } = await createMint(
        rpc,
        payer,
        mintAuthority,
        mintDecimals,
        mintKeypair,
      );
      loader.stop(false);
      console.log("\x1b[1mMint public key:\x1b[0m ", mint.toBase58());
      console.log(
        "\x1b[1mMint tx:\x1b[0m ",
        generateSolanaTransactionURL("tx", transactionSignature, "custom"),
      );
      console.log("create-mint successful");
    } catch (error) {
      this.error(`Failed to create-mint!\n${error}`);
    }
  }

  getMintDecimals(flags: any): number {
    return flags["mint-decimals"] ?? DEFAULT_DECIMAL_COUNT;
  }

  private async getMintKeypair(flags: any): Promise<Keypair | undefined> {
    const mintKeypairFilePath = flags["mint-keypair"];
    if (!mintKeypairFilePath) {
      return undefined;
    }
    const keypair = await getKeypairFromFile(mintKeypairFilePath);
    return keypair;
  }

  async getMintAuthority(flags: any, payer: Keypair): Promise<Keypair> {
    return flags["mint-authority"]
      ? await getKeypairFromFile(flags["mint-authority"])
      : payer;
  }
}

export default CreateMintCommand;
