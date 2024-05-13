import { Command, Flags } from "@oclif/core";
import { getKeypairFromFile } from "@solana-developers/helpers";
import {
  CustomLoader,
  defaultSolanaWalletKeypair,
  generateSolanaTransactionURL,
  getSolanaRpcUrl,
} from "../../utils/utils";

import { Keypair, PublicKey } from "@solana/web3.js";
import { getTestRpc } from "@lightprotocol/stateless.js";
import { registerMint } from "@lightprotocol/compressed-token";
import { WasmFactory } from "@lightprotocol/hasher.rs";

class RegisterMintCommand extends Command {
  static summary = "Register an existing mint with the CompressedToken program";

  static examples = ["$ light register-mint --mint-decimals 5"];

  static flags = {
    mint: Flags.string({
      description: "Provide a base58 encoded mint address to register",
      required: true,
    }),
    "mint-authority": Flags.string({
      description:
        "Specify a path to the mint authority keypair file. Defaults to your default local solana wallet file path",
      required: false,
    }),
  };

  static args = {};

  async run() {
    const { flags } = await this.parse(RegisterMintCommand);

    const loader = new CustomLoader(`Performing register-mint...\n`);
    loader.start();
    try {
      const payer = defaultSolanaWalletKeypair();
      const mintAddress = new PublicKey(flags.mint);
      const mintAuthority = await this.getMintAuthority(flags, payer);
      const lightWasm = await WasmFactory.getInstance();
      const rpc = await getTestRpc(lightWasm);
      const txId = await registerMint(rpc, payer, mintAuthority, mintAddress);
      loader.stop(false);
      console.log("\x1b[1mMint public key:\x1b[0m ", mintAddress.toBase58());
      console.log(
        "\x1b[1mMint tx:\x1b[0m ",
        generateSolanaTransactionURL("tx", txId, "custom"),
      );
      console.log("register-mint successful");
    } catch (error) {
      this.error(`Failed to register-mint!\n${error}`);
    }
  }

  async getMintAuthority(flags: any, payer: Keypair): Promise<Keypair> {
    return flags["mint-authority"]
      ? await getKeypairFromFile(flags["mint-authority"])
      : payer;
  }
}

export default RegisterMintCommand;
