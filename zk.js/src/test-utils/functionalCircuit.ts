import {
  FEE_ASSET,
  Account,
  Provider as LightProvider,
  MINT,
  Utxo,
  IDL_LIGHT_PSP2IN2OUT,
  lightPsp2in2outId,
  createTransaction,
  TransactionInput,
  getVerifierProgramId,
  getSystemProof,
  createSystemProofInputs,
  hashAndTruncateToCircuit,
  BN_0,
  getTransactionHash,
} from "../index";
import { WasmHasher } from "@lightprotocol/account.rs";
import { BN } from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";

export async function functionalCircuitTest(
  app: boolean = false,
  verifierIdl: Idl,
) {
  const lightProvider = await LightProvider.loadMock();
  const mockPubkey = SolanaKeypair.generate().publicKey;

  const hasher = await WasmHasher.getInstance();
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const account = Account.createFromSeed(hasher, seed32);
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;
  const relayerFee = new BN(5000);
  const inputUtxo = new Utxo({
    hasher: hasher,
    assets: [FEE_ASSET, MINT],
    amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
    publicKey: account.keypair.publicKey,
    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    index: 0,
    verifierAddress: app ? mockPubkey : undefined,
  });

  const merkleTree = new MerkleTree(18, hasher, [
    inputUtxo.getCommitment(hasher),
  ]);
  inputUtxo.merkleProof = merkleTree.path(0).pathElements;

  const outputUtxo1 = new Utxo({
    hasher: hasher,
    assets: [FEE_ASSET, MINT],
    amounts: [
      new BN(shieldFeeAmount / 2).sub(relayerFee),
      new BN(shieldAmount / 2),
    ],
    publicKey: account.keypair.publicKey,
    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
  });

  const outputUtxo2 = new Utxo({
    hasher: hasher,
    assets: [FEE_ASSET, MINT],
    amounts: [new BN(shieldFeeAmount / 2), new BN(shieldAmount / 2)],
    publicKey: account.keypair.publicKey,
    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
  });

  const txInput: TransactionInput = {
    inputUtxos: [inputUtxo],
    outputUtxos: [outputUtxo1, outputUtxo2],
    transactionMerkleTreePubkey: mockPubkey,
    hasher,
    account,
    relayerFee,
    systemPspId: getVerifierProgramId(verifierIdl),
    relayerPublicKey: lightProvider.relayer.accounts.relayerPubkey,
    pspId: app ? getVerifierProgramId(IDL_LIGHT_PSP2IN2OUT) : undefined,
  };

  const transaction = await createTransaction(txInput);
  let systemProofInputs = createSystemProofInputs({
    transaction: transaction,
    hasher,
    account,
    root: merkleTree.root(),
  });

  const transactionHash = getTransactionHash(
    hasher,
    transaction.private.inputUtxos,
    transaction.private.outputUtxos,
    BN_0, // is not checked in circuit
  );
  systemProofInputs = {
    ...systemProofInputs,
    publicAppVerifier: hashAndTruncateToCircuit(mockPubkey.toBytes()),
    transactionHash,
    txIntegrityHash: "0",
    internalTxIntegrityHash: "0",
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
    if (!error.toString().includes("CheckIndices_3 line: 34")) {
      throw new Error(
        "Expected error to be CheckIndices_3, but it was " + error.toString(),
      );
    }
  }
  if (!x) {
    throw new Error("Expected value to be true, but it was false.");
  }
}
