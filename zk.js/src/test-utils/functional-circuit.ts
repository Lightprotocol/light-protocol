import { OutUtxo, Utxo } from "../utxo";
import { WasmFactory } from "@lightprotocol/account.rs";
import { BN } from "@coral-xyz/anchor";
import { PublicKey, Keypair as SolanaKeypair } from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";

import {
  TransactionInput,
  createSystemProofInputs,
  getSystemProof,
  getTransactionHash,
  getVerifierProgramId,
} from "../transaction";
import { BN_0, FEE_ASSET, MERKLE_TREE_HEIGHT } from "../constants";
import { Account } from "../account";
import { Provider as LightProvider } from "../provider";
import { MINT } from "./constants-system-verifier";
import { createTransaction } from "../transaction";
import { hashAndTruncateToCircuit } from "../utils";
import { createOutUtxo, outUtxoToUtxo } from "../utxo";

export async function functionalCircuitTest(
  app: boolean = false,
  verifierIdl: Idl,
  pspId?: PublicKey,
  isPublic = false,
) {
  const lightProvider = await LightProvider.loadMock();
  const mockPubkey = SolanaKeypair.generate().publicKey;

  const lightWasm = await WasmFactory.getInstance();
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const account = Account.createFromSeed(lightWasm, seed32);
  const compressAmount = 20_000;
  const compressFeeAmount = 10_000;
  const rpcFee = new BN(5000);
  const _inputUtxo: OutUtxo | Utxo = createOutUtxo({
    lightWasm,
    assets: [FEE_ASSET, MINT],
    amounts: [new BN(compressFeeAmount), new BN(compressAmount)],
    owner: account.keypair.publicKey,
    isPublic,
  });

  const merkleTree = new MerkleTree(MERKLE_TREE_HEIGHT, lightWasm, [
    _inputUtxo.hash.toString(),
  ]);
  const inputUtxo = outUtxoToUtxo({
    outUtxo: _inputUtxo,
    merkleProof: merkleTree.path(0).pathElements,
    merkleTreeLeafIndex: 0,
    lightWasm,
    account,
  });

  const outputUtxo1 = createOutUtxo({
    lightWasm,
    assets: [FEE_ASSET, MINT],
    amounts: [
      new BN(compressFeeAmount / 2).sub(rpcFee),
      new BN(compressAmount / 2),
    ],
    owner: account.keypair.publicKey,
    isPublic,
  });

  const outputUtxo2 = createOutUtxo({
    lightWasm,
    assets: [FEE_ASSET, MINT],
    amounts: [new BN(compressFeeAmount / 2), new BN(compressAmount / 2)],
    owner: account.keypair.publicKey,
    isPublic,
  });

  const txInput: TransactionInput = {
    inputUtxos: [inputUtxo],
    outputUtxos: [outputUtxo1, outputUtxo2],
    merkleTreeSetPubkey: mockPubkey,
    lightWasm,
    account,
    rpcFee,
    systemPspId: getVerifierProgramId(verifierIdl),
    rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
    pspId: app ? pspId : undefined,
  };

  const transaction = await createTransaction(txInput);
  let systemProofInputs = createSystemProofInputs({
    transaction: transaction,
    lightWasm,
    account,
    root: merkleTree.root(),
  });

  const publicTransactionHash = getTransactionHash(
    transaction.private.inputUtxos,
    transaction.private.outputUtxos,
    BN_0, // is not checked in circuit
    lightWasm,
  );
  systemProofInputs = {
    ...systemProofInputs,
    publicProgramId: hashAndTruncateToCircuit(
      [Uint8Array.from(mockPubkey.toBytes())],
      lightWasm,
    ),
    publicTransactionHash,
    privatePublicDataHash: "0",
    publicDataHash: "0",
  } as any;

  // we rely on the fact that the function throws an error if proof generation failed
  await getSystemProof({
    account,
    inputUtxos: transaction.private.inputUtxos,
    verifierIdl,
    systemProofInputs,
  });

  // unsuccessful proof generation
  let x = true;

  try {
    systemProofInputs.inIndices[0][1][1] = "1";
    // TODO: investigate why this does not kill the proof
    systemProofInputs.inIndices[0][1][0] = "1";
    const systemProof = await getSystemProof({
      account,
      inputUtxos: transaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });
    x = false;
  } catch (error: any) {
    if (!error.toString().includes("CheckIndices_") && !app) {
      throw new Error(
        "Expected error to be CheckIndices_3, but it was " + error.toString(),
      );
    }

    if (!error.toString().includes("CheckIndices_") && app) {
      throw new Error(
        "Expected error to be CheckIndices_5, but it was " + error.toString(),
      );
    }
  }
  if (!x) {
    throw new Error("Expected value to be true, but it was false.");
  }
}
