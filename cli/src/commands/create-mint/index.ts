import { Command, Flags } from "@oclif/core";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getPayer,
  getSolanaRpcUrl,
} from "../../utils/utils";
import { createMint } from "@lightprotocol/compressed-token";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";

const DEFAULT_DECIMAL_COUNT = 9;

class CreateMintCommand extends Command {
  static summary = "Create a new compressed token mint";

  static examples = ["$ light create-mint --mint-decimals 5"];

  static flags = {
    "mint-secret-key": Flags.string({
      description: "Provide the mint secret key to use for minting",
      required: false,
    }),
    "mint-authority": Flags.string({
      description:
        "Specify the mint authority address. Defaults to the client keypair address",
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
    const { args, flags } = await this.parse(CreateMintCommand);

    const loader = new CustomLoader(`Performing create-mint...\n`);
    loader.start();
    try {
      const payer = defaultSolanaWalletKeypair();
      const mintDecimals = this.getMintDecimals(flags);
      const mintKeypair = this.getMintKeypair(flags);
      const mintAuthority = this.getMintAuthority(flags, payer);
      const connection = new Connection(getSolanaRpcUrl());
      const { mint, transactionSignature } = await createMint(
        connection,
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

  getMintKeypair(flags: any): Keypair | undefined {
    const mint58: string | undefined = flags["mint-secret-key"];
    return mint58 ? Keypair.fromSecretKey(bs58.decode(mint58)) : undefined;
  }

  getMintAuthority(flags: any, payer: any) {
    return flags["mint-authority"]
      ? new PublicKey(flags["mint-authority"])
      : payer.publicKey;
  }
}

export default CreateMintCommand;
