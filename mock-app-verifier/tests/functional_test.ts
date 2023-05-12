import * as anchor from "@coral-xyz/anchor";

import {
  Utxo,
  Transaction,
  ADMIN_AUTH_KEYPAIR,
  initLookUpTableFromFile,
  setUpMerkleTree,
  createTestAccounts,
  KEYPAIR_PRIVKEY,
  Account,
  TRANSACTION_MERKLE_TREE_KEY,
  TransactionParameters,
  Provider as LightProvider,
  userTokenAccount,
  ADMIN_AUTH_KEY,
  confirmConfig,
  Action,
  TestRelayer,
  hashAndTruncateToCircuit,
  createAccountObject,
  TestTransaction,
  IDL_VERIFIER_PROGRAM_TWO,
  User,
  airdropShieldedSol,
  ProgramUtxoBalance,
  MINT,
  airdropShieldedMINTSpl,
} from "light-sdk";
import {
  Keypair as SolanaKeypair,
  SystemProgram,
  PublicKey,
} from "@solana/web3.js";
import { marketPlaceVerifierProgramId, MockVerifier } from "../sdk/src/index";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { IDL } from "../target/types/mock_verifier";
import { assert, expect } from "chai";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

var POSEIDON, LOOK_UP_TABLE, RELAYER, KEYPAIR, relayerRecipientSol: PublicKey ,outputUtxoSpl: Utxo, outputUtxoSol: Utxo;
const performStoreAppUtxo = async (seed: string, testInputs: any, airdrop: boolean) => {
  const lightProvider = await LightProvider.init({
    wallet: ADMIN_AUTH_KEYPAIR,
    relayer: RELAYER,
  });
  const user: User = await User.init({ provider: lightProvider, seed });

  await user.getBalance();
  if(airdrop) {
    if(testInputs.utxo.amounts[0]) {
      await airdropShieldedSol({seed: testInputs.seed, amount: (testInputs.utxo.amounts[0].div(new BN(1e9))).toNumber()});
    }
  }
  await user.storeAppUtxo({
    appUtxo: testInputs.utxo,
    action: testInputs.action,
  });
  const res: Map<string, ProgramUtxoBalance> = await user.syncStorage(IDL);
  Utxo.equal(testInputs.poseidon, res.get(marketPlaceVerifierProgramId.toBase58()).tokenBalances.get(testInputs.utxo.assets[1].toBase58()).utxos.get(testInputs.utxo.getCommitment(testInputs.poseidon)), testInputs.utxo);
}
describe("Mock verifier functional", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  const path = require("path");
  const circuitPath = path.resolve(__dirname, "../sdk/build-circuit/");

  anchor.setProvider(provider);
  var poseidon, account: Account, outputUtxo: Utxo;
  const seed = bs58.encode(new Uint8Array(32).fill(1));

  before(async () => {
    poseidon = await buildPoseidonOpt();

    console.log("Initing accounts");
    await createTestAccounts(provider.connection, userTokenAccount);
    POSEIDON = await buildPoseidonOpt();
    KEYPAIR = new Account({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });
    await setUpMerkleTree(provider);
    LOOK_UP_TABLE = await initLookUpTableFromFile(
      provider,
      "lookUpTable.txt" /*Array.from([relayerRecipientSol])*/,
    );

    relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000,
    );

    RELAYER = await new TestRelayer(
      ADMIN_AUTH_KEYPAIR.publicKey,
      LOOK_UP_TABLE,
      relayerRecipientSol,
      new BN(100000),
    );
    account = new Account({
      poseidon,
      seed: bs58.encode(new Uint8Array(32).fill(1)),
    });
    outputUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      account,
      amounts: [new BN(1_000_000)],
      appData: { testInput1: new BN(1), testInput2: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: marketPlaceVerifierProgramId,
      index: 0,
    });
    outputUtxoSol = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      account,
      amounts: [new BN(1_12321211)],
      appData: { testInput1: new BN(1), testInput2: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: marketPlaceVerifierProgramId,
      index: 0,
    });
    outputUtxoSpl = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, MINT],
      account,
      amounts: [new BN(1_000_000), new BN(1234)],
      appData: { testInput1: new BN(1), testInput2: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: marketPlaceVerifierProgramId,
      index: 0,
    });
  });

  it("To from bytes ", async () => {
    let bytes = await outputUtxo.toBytes();

    let utxo1 = Utxo.fromBytes({
      poseidon,
      bytes,
      index: 0,
      account,
      appDataIdl: IDL,
    });
    Utxo.equal(poseidon, outputUtxo, utxo1);
  });

  it("Pick app data from utxo data", () => {
    let data = createAccountObject(
      {
        testInput1: 1,
        testInput2: 2,
        rndOtherStuff: { s: 2342 },
        o: [2, 2, new BN(2)],
      },
      IDL.accounts,
      "utxoAppData",
    );
    assert.equal(data.testInput1, 1);
    assert.equal(data.testInput2, 2);
    assert.equal(data.rndOtherStuff, undefined);
    assert.equal(data.o, undefined);

    expect(() => {
      createAccountObject(
        { testInput1: 1, rndOtherStuff: { s: 2342 }, o: [2, 2, new BN(2)] },
        IDL.accounts,
        "utxoAppData",
      );
    }).to.throw(Error);
  });

  it("sol 1 ", async () =>{
    await airdropShieldedSol({
      amount: 2,
      seed
    });
    const testInputsSol1 = {
      utxo: outputUtxoSol,
      action: Action.TRANSFER,
      poseidon
    }

    await performStoreAppUtxo(
      seed,
      testInputsSol1,
      false
    )
  })

  it("spl ", async () =>{

    await airdropShieldedMINTSpl({
      amount: outputUtxoSpl.amounts[1].toNumber(),
      seed
    });

    const testInputsSpl = {
      utxo: outputUtxoSpl,
      action: Action.TRANSFER,
      poseidon
    }

    await performStoreAppUtxo(
      seed,
      testInputsSpl,
      false
    )
  })

  // TODO: add storage verifier transactionNonce -> make transaction nonce a map<MerkleTreePubkey, transactionNonce>
  // TODO: add getAll programUtxos
  // TODO: replace compressed flag in encrypt/decrypt with includeProgramData
  // TODO: cleanup -> do pr -> do build process for
  it("Test Deposit MockVerifier cpi VerifierTwo", async () => {
    let lightProvider = await LightProvider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });

    const txParams = new TransactionParameters({
      outputUtxos: [outputUtxo],
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      senderSpl: userTokenAccount, // just any token account
      senderSol: ADMIN_AUTH_KEY, //
      lookUpTable: LOOK_UP_TABLE,
      poseidon,
      action: Action.SHIELD,
      encryptedUtxos: new Uint8Array(256).fill(1),
      transactionNonce: 0,
      verifierIdl: IDL_VERIFIER_PROGRAM_TWO,
    });

    const appParams = {
      verifier: new MockVerifier(),
      inputs: { releaseSlot: new BN(1), currentSlot: new BN(1) },
      path: circuitPath,
      verifierIdl: IDL,
    };

    let transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
      appParams,
    });

    await transactionTester.getTestValues();

    let tx = new Transaction({
      provider: lightProvider,
      params: txParams,
      appParams,
    });

    await tx.compile();
    await tx.provider.provider.connection.confirmTransaction(
      await tx.provider.provider.connection.requestAirdrop(
        tx.params.accounts.authority,
        1_000_000_000,
      ),
    );
    await tx.getProof();
    await tx.getAppProof();
    await tx.sendAndConfirmTransaction();
    await transactionTester.checkBalances(
      tx.transactionInputs,
      tx.remainingAccounts,
      tx.proofInput,
      KEYPAIR,
    );
    await lightProvider.relayer.updateMerkleTree(lightProvider);
  });

  it.only("Test Withdrawal MockVerifier cpi VerifierTwo", async () => {
    const poseidon = await buildPoseidonOpt();

    let lightProvider = await LightProvider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(relayerRecipientSol, 10000000),
    );

    // TODO: add check that recipients are defined if withdrawal
    const txParams = new TransactionParameters({
      inputUtxos: [outputUtxo],
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      recipientSpl: userTokenAccount, // just any token account
      recipientSol: SolanaKeypair.generate().publicKey,
      action: Action.UNSHIELD,
      poseidon,
      relayer: RELAYER,
      transactionNonce: 1,
      verifierIdl: IDL_VERIFIER_PROGRAM_TWO,
    });

    const appParams = {
      verifier: new MockVerifier(),
      inputs: { releaseSlot: new BN(1), currentSlot: new BN(1) },
      path: circuitPath,
      verifierIdl: IDL,
    };
    let transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
      appParams,
    });
    await transactionTester.getTestValues();

    let tx = new Transaction({
      provider: lightProvider,
      params: txParams,
      appParams,
    });

    await tx.compile();
    await tx.getProof();
    await tx.getAppProof();
    await tx.sendAndConfirmTransaction();
    await transactionTester.checkBalances(
      tx.transactionInputs,
      tx.remainingAccounts,
      tx.proofInput,
      KEYPAIR,
    );
  });
});
