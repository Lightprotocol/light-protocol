import { Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

export class Relayer {
  accounts: {
    relayerPubkey: PublicKey; // signs the transaction
    relayerRecipient: PublicKey; // receives the fees
    lookUpTable: PublicKey;
  };
  relayerFee: BN;

  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipient?: PublicKey,
    relayerFee: BN = new BN(0)
  ) {
    if (relayerRecipient) {
      this.accounts = {
        relayerPubkey,
        lookUpTable,
        relayerRecipient,
      };
    } else {
      this.accounts = {
        relayerPubkey,
        lookUpTable,
        relayerRecipient: relayerPubkey,
      };
    }
    this.relayerFee = relayerFee;
  }
}
