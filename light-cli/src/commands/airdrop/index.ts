import { Args, Command, Flags } from "@oclif/core";
import { PublicKey, RpcResponseAndContext, SignatureResult } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  MINT,
  convertAndComputeDecimals,
  airdropSplToAssociatedTokenAccount
  } from "@lightprotocol/zk.js";
import {
  CustomLoader,
  generateSolanaTransactionURL,
  setAnchorProvider,
} from "../../utils/utils";

class AirdropCommand extends Command {
  static description = "Perform a native Solana or SPL airdrop to a user";
  static examples = [
    `$ light airdrop 1.5 <RECIPIENT_ADDRESS>`,
    `$ light airdrop --token USDC 15 <RECIPIENT_ADDRESS> -v`,
  ];

  static flags = {
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol",
      default: "SOL",
      parse: async (token) => token.toUpperCase(), 
    }),
    verbose: Flags.boolean({
      char: "v",
      description: "Show additional information",
      default: false,
    }),
  };

  static args = {
    amount: Args.string({
      name: "AMOUNT",
      description: "The airdrop amount to request",
      required: true,
    }),
    recipient_address: Args.string({
      name: "RECIPIENT_ADDRESS",
      description: "The account address of recipient",
      required: true,
    }),
  };

  async run() {
    const { args, flags } = await this.parse(AirdropCommand);
    const amount = args.amount;
    const recipient_address = new PublicKey(args.recipient_address);
    const { token, verbose } = flags;

    const loader = new CustomLoader(`Requesting airdrop of ${amount} ${token}...`);
    loader.start();

    let transactionSignature: string;
    let transactionInfo: RpcResponseAndContext<SignatureResult>;

    try {
      const provider = await setAnchorProvider();

      if (token.toLowerCase() === "sol") {
        transactionSignature = await provider.connection.requestAirdrop(
          recipient_address, 
          convertAndComputeDecimals(amount, new BN(1e9)).toNumber()
        );
        transactionInfo = await provider.connection.confirmTransaction(
          transactionSignature,
          "confirmed",
        );
        
      } else {
        transactionSignature = await airdropSplToAssociatedTokenAccount(
          provider.connection,
          parseInt(amount) * 100,
          recipient_address
        );
  
        transactionInfo = await provider.connection.confirmTransaction(
          transactionSignature,
          "confirmed",
        );
      } 

      if (verbose) {
        this.log(`
        ===========================
        =     \x1b[35mAirdrop Summary\x1b[0m     =
        ===========================
        `);
        this.log(`\x1b[34mRecipient\x1b[0m: ${recipient_address}`);
        this.log(`\x1b[34mToken\x1b[0m:     ${token}`);
        this.log(`\x1b[34mAmount\x1b[0m:    ${amount}`);
        if (token.toLowerCase() !== "sol") {
          this.log(`\x1b[34mMint:\x1b[0m      ${MINT}`);
        }

        this.log(`
        ===========================
        = \x1b[35mTransaction Information\x1b[0m =
        ===========================
        `);
        this.log(`\x1b[34mTransaction signature\x1b[0m: ${transactionSignature}`);
        this.log(`\x1b[34mBlock number\x1b[0m:          ${transactionInfo.context.slot}`);
        this.log(`\x1b[34mTransaction status\x1b[0m:    ${transactionInfo.value.err ? 'failed' : 'success'}`);

        this.log(`\nYou can view more transaction details at:`);
        this.log(`${generateSolanaTransactionURL("tx", transactionSignature!, "custom")}`);
        this.log("\nAirdrop Successful \x1b[32m✔\x1b[0m");
      }
      else {
        this.log(`\n\x1b[1mRecipient:\x1b[0m ${recipient_address}`);
        this.log(`\x1b[1mSignature:\x1b[0m ${transactionSignature}`);
        if (token.toLowerCase() !== "sol") this.log(`\x1b[1mMint:\x1b[0m      ${MINT}`);
        this.log(generateSolanaTransactionURL("tx", transactionSignature!, "custom"));
        this.log("\nAirdrop Successful \x1b[32m✔\x1b[0m");
      }  
      
      loader.stop(false);
    } catch (error) {
      this.error(`Failed to airdrop ${token}!\n${error}`);
    }
  }
}

export default AirdropCommand;
