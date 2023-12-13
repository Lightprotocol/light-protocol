import { BN, BorshAccountsCoder } from "@coral-xyz/anchor";
import {
  Provider,
  LegacyTransaction as Transaction,
  TransactionParameters,
} from "@lightprotocol/zk.js";

export type QueuedTransactionsType = {
  transactionParams: TransactionParameters;
  approvals: {
    publicKey: Uint8Array[][];
    signature: Uint8Array[];
  };
};
import { IDL } from "./types/multisig";
import { printUtxo } from "./client";

export class Approval {
  signature: Uint8Array;
  signerIndex: number;
  publicKey: [Uint8Array, Uint8Array];
  constructor({
    signerIndex,
    publicKey,
    signature,
  }: {
    signerIndex: number;
    publicKey: [Uint8Array, Uint8Array];
    signature: Uint8Array;
  }) {
    this.publicKey = publicKey;
    this.signature = signature;
  }
  async toBytes(): Promise<Buffer> {
    const coder = new BorshAccountsCoder(IDL);
    return coder.encode("approveTransaction", this);
  }

  static fromBytes(bytes: Buffer): Approval {
    const coder = new BorshAccountsCoder(IDL);
    let decoded = coder.decode("approveTransaction", bytes);
    return new Approval({
      signerIndex: decoded.signerIndex,
      publicKey: decoded.publicKey,
      signature: decoded.signature,
    });
  }
}

export class QueuedTransaction {
  transactionParams: TransactionParameters;
  approvals: Approval[];

  constructor(transactionParams: TransactionParameters) {
    this.transactionParams = transactionParams;
    this.approvals = [];
  }

  addApproval(approval: Approval) {
    this.approvals.push(approval);
  }

  async print(poseidon: any) {
    let print = "";

    console.log(`-------------- Input Utxos --------------\n`);

    for (var utxo in this.transactionParams.inputUtxos) {
      console.log(
        printUtxo(
          this.transactionParams.inputUtxos[utxo],
          poseidon,
          Number(utxo) + 1,
          "input",
        ),
      );
    }
    console.log("\n\n");
    console.log(`-------------- Output Utxos --------------\n`);

    for (var utxo in this.transactionParams.outputUtxos) {
      console.log(
        printUtxo(
          this.transactionParams.outputUtxos[utxo],
          poseidon,
          Number(utxo) + 1,
          "output",
        ),
      );
    }
    console.log("\n\n");
    print += "-------------- Public Transaction Parameters --------------\n";
    print +=
      "recipient spl " +
      this.transactionParams.accounts.recipientSpl.toBase58() +
      "\n";
    print +=
      "recipient sol " +
      this.transactionParams.accounts.recipientSol.toBase58() +
      "\n";
    print +=
      "relayer " +
      this.transactionParams.relayer.accounts.relayerPubkey.toBase58() +
      "\n";
    print +=
      "relayer fee " +
      this.transactionParams.relayer.relayerFee
        .div(new BN(1_000_000_000))
        .toString() +
      "\n";
    print +=
      "encrypted utxos length " +
      this.transactionParams.encryptedUtxos.length +
      "\n";
    print += "------------------------------------------";

    console.log(print);

    let provider: Provider = await Provider.loadMock();
    let { rootIndex, remainingAccounts } = await provider.getRootIndex();
    let tx = new Transaction({
      rootIndex,
      ...remainingAccounts,
      solMerkleTree: provider.solMerkleTree!,
      params: this.transactionParams,
    });

    const connectingHash = this.transactionParams.getTransactionHash(
      provider.hasher,
    );

    console.log(`-------------- Shielded Transaction Hash --------------\n`);
    console.log(connectingHash.toString());
    console.log("------------------------------------------");
  }
}
