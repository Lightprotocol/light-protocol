import { PublicKey } from "@solana/web3.js";
import {
  CustomLoader,
  getWalletConfig,
  setAnchorProvider,
} from "../../utils/utils";
import { Args, Command } from "@oclif/core";

class VerifierRegisterCommand extends Command {
  static description = "Register a verifier.";

  static examples = ["light merkle-tree-authority:verifier-register"];

  static args = {
    verifier: Args.string({
      description: "Solana public key of the verifier to register.",
      required: true,
    }),
  };

  async run() {
    const { args } = await this.parse(VerifierRegisterCommand);
    const { verifier } = args;

    const anchorProvider = await setAnchorProvider();
    const merkleTreeConfig = await getWalletConfig(anchorProvider);

    const verifierKey = new PublicKey(verifier);

    const loader = new CustomLoader(`Registering verifier ${verifier}`);

    await merkleTreeConfig.registerVerifier(verifierKey);
    this.log("Verifier registered successfully \x1b[32m✔\x1b[0m");
    loader.stop(false);
  }
}

export default VerifierRegisterCommand;
