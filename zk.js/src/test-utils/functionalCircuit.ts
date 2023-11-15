import {
  FEE_ASSET,
  Account,
  Provider as LightProvider,
  MINT,
  Utxo,
  Transaction,
  Action,
  TransactionParameters,
  IDL_LIGHT_PSP2IN2OUT,
  lightPsp2in2outId,
} from "../index";
import { Poseidon } from "@lightprotocol/account.rs";
import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

export async function functionalCircuitTest(
  app: boolean = false,
  verifierIdl: Idl,
) {
  const lightProvider = await LightProvider.loadMock();

  const poseidon = await Poseidon.getInstance();
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const account = new Account({ poseidon: poseidon, seed: seed32 });
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;
  const shieldUtxo1 = new Utxo({
    poseidon: poseidon,
    assets: [FEE_ASSET, MINT],
    amounts: [new anchor.BN(shieldFeeAmount), new anchor.BN(shieldAmount)],
    publicKey: account.pubkey,
    assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    verifierAddress: lightPsp2in2outId,
  });
  const mockPubkey = SolanaKeypair.generate().publicKey;

  const txParams = new TransactionParameters({
    outputUtxos: [shieldUtxo1],
    eventMerkleTreePubkey: mockPubkey,
    transactionMerkleTreePubkey: mockPubkey,
    senderSpl: mockPubkey,
    senderSol: lightProvider.wallet!.publicKey,
    action: Action.SHIELD,
    poseidon,
    verifierIdl: verifierIdl,
    account,
  });

  let tx: Transaction;
  const { rootIndex, remainingAccounts } = await lightProvider.getRootIndex();
  // successful proof generation
  if (app) {
    tx = new Transaction({
      rootIndex,
      nextTransactionMerkleTree: remainingAccounts.nextTransactionMerkleTree,
      solMerkleTree: lightProvider.solMerkleTree!,
      params: txParams,
      appParams: {
        mock: "123",
        // just a placeholder the test does not compute an app proof
        verifierIdl: IDL_LIGHT_PSP2IN2OUT,
        path: "./build-circuits",
      },
    });
  } else {
    tx = new Transaction({
      rootIndex,
      nextTransactionMerkleTree: remainingAccounts.nextTransactionMerkleTree,
      solMerkleTree: lightProvider.solMerkleTree!,
      params: txParams,
    });
  }
  await tx.compile(lightProvider.poseidon, account);

  await tx.getProof(account);
  // unsuccessful proof generation
  let x = true;

  try {
    tx.proofInput.inIndices[0][1][1] = "1";
    // TODO: investigate why this does not kill the proof
    tx.proofInput.inIndices[0][1][0] = "1";
    await tx.getProof(account);
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
