import { Args, Command, Flags } from "@oclif/core";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  rpc,
} from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";
import {
  createAtaInterfaceIdempotent,
  decompressMint,
  getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token";

class CreateTokenAccountCommand extends Command {
  static summary =
    "Create an associated token account for a given mint and owner.";

  static examples = [
    "$ light create-token-account <MINT>",
    "$ light create-token-account <MINT> --owner <OWNER>",
  ];

  static args = {
    mint: Args.string({
      description: "Base58 encoded mint address.",
      required: true,
    }),
  };

  static flags = {
    owner: Flags.string({
      description:
        "Owner of the token account. Defaults to the fee payer's public key.",
      required: false,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(CreateTokenAccountCommand);

    const loader = new CustomLoader(
      `Performing create-token-account...\n`,
    );
    loader.start();

    try {
      const payer = defaultSolanaWalletKeypair();
      const mintPublicKey = new PublicKey(args.mint);
      const ownerPublicKey = flags.owner
        ? new PublicKey(flags.owner)
        : payer.publicKey;

      try {
        await decompressMint(rpc(), payer, mintPublicKey);
      } catch {
        // Mint may already be decompressed; ignore.
      }

      await createAtaInterfaceIdempotent(
        rpc(),
        payer,
        mintPublicKey,
        ownerPublicKey,
      );

      const ataAddress = getAssociatedTokenAddressInterface(
        mintPublicKey,
        ownerPublicKey,
      );

      loader.stop(false);
      console.log(
        "\x1b[1mToken account:\x1b[0m ",
        ataAddress.toBase58(),
      );
      console.log(
        "\x1b[1mAccount address:\x1b[0m ",
        generateSolanaTransactionURL(
          "address",
          ataAddress.toBase58(),
          "custom",
        ),
      );
      console.log("create-token-account successful");
    } catch (error) {
      this.error(`Failed to create-token-account!\n${error}`);
    }
  }
}

export default CreateTokenAccountCommand;
