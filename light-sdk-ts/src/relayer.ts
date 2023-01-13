import { Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

export class Relayer {
  relayerPubkey: PublicKey; // signs the transaction
  relayerRecipient: PublicKey; // receives the fees
  lookUpTable: PublicKey;
  relayerFee: BN;

  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipient?: PublicKey,
    relayerFee: BN = new BN(0)
  ) {
    this.relayerPubkey = relayerPubkey;
    if (!relayerRecipient) {
      this.relayerRecipient = relayerPubkey;
    } else {
      this.relayerRecipient = relayerRecipient;
    }
    this.lookUpTable = lookUpTable;
    this.relayerFee = relayerFee;
  }
}
