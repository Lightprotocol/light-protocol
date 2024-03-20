import { Command, Flags } from "@oclif/core";
import { initTestEnv } from "../../utils/initTestEnv";
import { CustomLoader } from "../../utils/index";
import { provingArgs, verifyingArgs } from "../../utils/proverUtils";
import { execute } from "../../psp-utils";
class VerifyCommand extends Command {
  static description = "Verify a proof of inclusion of a leaf in a Merkle tree";

  static examples = [
    `$ light verify --roots "0x1ebf5c4eb04bf878b46937be63d12308bb14841813441f041812ea54ecb7b2d5" --leafs "0x29176100eaa962bdc1fe6c654d6a3c130e96a4d1168b33848b897dc502820133" --proof '{"ar":["0x2e16973b22414e43e99d8240045a1919063f31caf5a2d260f7840b6b5969a73f","0x1872024e8c1b30063914cb428bfb4e975d62b69853976614f74af7374cf9b184"],"bs":[["0x8ac1ec150a0f9244de332d2becebd31c33619b79d6d40647e4d0f1b73e12d8","0xdc42af00fdee64eeaf34d766ea9ff5365bc6756525de25707b4c917acedeea0"],["0x2ffd42f088ae2290ba0e206fc5102a4f017fed0cb019b618f54d54ee5033c425","0x2f26212f5e71c2b2973c01359e7e78ca1d36961d1d883d3db79e454b39794483"]],"krs":["0x1c015d1b7283f4caa5a8e4fbffc4316e10ee0855a5561564c3f07648246fcb4","0x1f35f1de13b684ebe6d41eb75075210e05d5875feea25edc8882c8fcb59c1c55"]}'`,
  ];

  static flags = {
    roots: Flags.string({
      char: "r",
      description: "Array of roots",
      default: "",
    }),
    leafs: Flags.string({
      char: "l",
      description: "Array of leafs",
      default: "",
    }),
    proof: Flags.string({
      char: "p",
      description: "The proof",
      default: "",
    }),
  };
  async run() {
    const { flags } = await this.parse(VerifyCommand);

    const loader = new CustomLoader("Verifying...\n");
    loader.start();

    const roots: string[] = flags.roots.split(",");
    const leafs: string[] = flags.leafs.split(",");
    const proof = flags.proof;

    if (roots.length === 0 || leafs.length === 0 || proof === "") {
      this.log("Verify failed: invalid input");
      loader.stop(false);
      return;
    }
    const result = await this.verify(proof, roots, leafs);
    if (result.trim() === "") {
      this.log("Verify failed");
      loader.stop(false);
      return;
    }
    console.log("\x1b[1mVerified:\x1b[0m ", result);
    this.log("\nVerified successfully \x1b[32m✔\x1b[0m");
    loader.stop(false);
  }

  async verify(proof: string, roots: string[], leafs: string[]) {
    const args = verifyingArgs(proof, roots, leafs);
    const result = await execute(args);
    return result;
  }
}

export default VerifyCommand;
