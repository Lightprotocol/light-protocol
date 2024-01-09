import * as anchor from "@coral-xyz/anchor";
import {
  Provider as LightProvider,
  confirmConfig,
  Action,
  TestRpc,
  User,
  airdropSol,
  STANDARD_SHIELDED_PUBLIC_KEY,
  PspTransactionInput,
  MerkleTreeConfig,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  createProofInputs,
  getSystemProof,
  setUndefinedPspCircuitInputsToZero,
  SolanaTransactionInputs,
  Provider,
  sendAndConfirmShieldedTransaction,
  hashAndTruncateToCircuit,
  createTransaction,
  lightPsp4in4outAppStorageId,
  syncInputUtxosMerkleProofs,
  shieldProgramUtxo,
  Utxo,
  createFillingUtxo,
  createProgramOutUtxo,
  createOutUtxo,
} from "@lightprotocol/zk.js";

import { SystemProgram, PublicKey, Keypair, Connection } from "@solana/web3.js";
import { LightWasm, WasmFactory } from "@lightprotocol/account.rs";
import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/swaps";
const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
);
import { assert } from "chai";

let WASM: LightWasm, RPC: TestRpc;
const RPC_URL = "http://127.0.0.1:8899";
const createTestUser = async (
  connection: Connection,
  lamports: number,
  shieldedSol?: number,
): Promise<User> => {
  let sellerWallet = Keypair.generate();
  await airdropSol({
    connection,
    lamports,
    recipientPublicKey: sellerWallet.publicKey,
  });
  const lightProvider: Provider = await LightProvider.init({
    wallet: sellerWallet,
    url: RPC_URL,
    rpc: RPC,
    confirmConfig,
  });
  let user: User = await User.init({ provider: lightProvider });
  if (shieldedSol) {
    // TODO: return utxo commitment hash
    await user.shield({
      token: "SOL",
      publicAmountSol: shieldedSol,
    });
  }
  return user;
};

describe("Test swaps", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    WASM = await WasmFactory.getInstance();
    const rpcWallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e11,
      recipientPublicKey: rpcWallet.publicKey,
    });
    RPC = new TestRpc({
      rpcPubkey: rpcWallet.publicKey,
      rpcRecipientSol: rpcWallet.publicKey,
      rpcFee: new BN(100000),
      payer: rpcWallet,
      connection: provider.connection,
      lightWasm: WASM,
    });
  });

  it("Swap Take functional", async () => {
    /**
     * 1. Create seller and buyer Users
     * 2. seller user creates offer
     *    - creates utxo
     *    - encrypts it to the buyer
     *    - stores the encrypted utxo onchain in a compressed account
     * 3. recipient decrypts offer
     * 4. recipient generates
     */
    const sellerUser: User = await createTestUser(provider.connection, 10e9);
    const buyerUser: User = await createTestUser(provider.connection, 10e9, 5);
    console.log(
      "new BN(sellerUser.account.encryptionKeypair.publicKey) ",
      new BN(sellerUser.account.encryptionKeypair.publicKey),
    );
    // TODO: add sorting to compute utxo data hash consistently
    // TODO: remove include appdata
    let offerUtxo = createProgramOutUtxo({
      lightWasm: WASM,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: buyerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e9)],
      utxoData: {
        priceSol: new BN(2e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: sellerUser.account.keypair.publicKey,
        recipientEncryptionPublicKey: hashAndTruncateToCircuit(
          sellerUser.account.encryptionKeypair.publicKey,
        ),
      },
      pspIdl: IDL,
      pspId: verifierProgramId,
      utxoName: "utxo",
    });

    const txHashMakeOffer = await shieldProgramUtxo({
      account: sellerUser.account,
      appUtxo: offerUtxo,
      provider: sellerUser.provider,
    });
    console.log("made offer: ", txHashMakeOffer);

    let syncedStorage = await buyerUser.syncStorage(IDL, false);
    //TODO: refactor to only have one program utxo layer then an utxo array
    let fetchedOfferUtxo: Utxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];

    assert.deepEqual(
      JSON.stringify(offerUtxo.outUtxo.utxoData),
      JSON.stringify(fetchedOfferUtxo.utxoData),
    );

    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.utxoData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.utxoData.recipient.toString()}`,
    );
    const circuitPath = path.join("build-circuit/swaps/swaps");
    await buyerUser.getBalance();
    const shieldUtxo = buyerUser.getAllUtxos()[0];

    // TODO: throw error if the pubkey is not mine and there is no encryption key specified
    const offerRewardUtxo = createOutUtxo({
      lightWasm: WASM,
      publicKey: fetchedOfferUtxo.utxoData.recipient,
      encryptionPublicKey: sellerUser.account.encryptionKeypair.publicKey,
      // TODO: Make this utxo works with:
      // Uint8Array.from(
      //   fetchedOfferUtxo.utxoData.recipientEncryptionPublicKey.toArray(),
      // ),
      amounts: [new BN(2e9)],
      assets: [SystemProgram.programId],
      blinding: new BN(fetchedOfferUtxo.blinding),
    });
    console.log(
      "fetchedOfferUtxo blinding: ",
      fetchedOfferUtxo.blinding.toString(),
    );
    console.log(
      "offerRewardUtxo blinding: ",
      offerRewardUtxo.blinding.toString(),
    );
    const tradeOutputUtxo = createOutUtxo({
      lightWasm: WASM,
      publicKey: fetchedOfferUtxo.utxoData.recipient,
      amounts: [new BN(1e9)],
      assets: [SystemProgram.programId],
    });

    const changeAmountSol = shieldUtxo.amounts[0]
      .sub(offerRewardUtxo.amounts[0])
      .sub(RPC.rpcFee);

    // TODO: add function to create change utxo
    const changeUtxo = createOutUtxo({
      lightWasm: WASM,
      publicKey: fetchedOfferUtxo.utxoData.recipient,
      amounts: [changeAmountSol],
      assets: [SystemProgram.programId],
    });

    // should I bundle it here or go through this step by step?
    // TODO: abstraction that unifies Transaction creation
    const pspTransactionInput: PspTransactionInput = {
      proofInputs: {
        takeOfferInstruction: new BN(1),
      },
      path: circuitPath,
      verifierIdl: IDL,
      circuitName: "swaps",
      checkedInUtxos: [{ utxoName: "offerUtxo", utxo: fetchedOfferUtxo }],
      checkedOutUtxos: [{ utxoName: "offerRewardUtxo", utxo: offerRewardUtxo }],
      inUtxos: [shieldUtxo],
      outUtxos: [changeUtxo, tradeOutputUtxo],
    };

    const {
      syncedUtxos: inputUtxos,
      root,
      index: rootIndex,
    } = await syncInputUtxosMerkleProofs({
      inputUtxos: [fetchedOfferUtxo, shieldUtxo],
      rpc: RPC,
      merkleTreePublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });
    const outputUtxos = [changeUtxo, tradeOutputUtxo, offerRewardUtxo];

    const shieldedTransaction = await createTransaction({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      rpcPublicKey: RPC.accounts.rpcPubkey,
      lightWasm: WASM,
      rpcFee: RPC.rpcFee,
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: buyerUser.account,
    });

    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const proofInputs = createProofInputs({
      lightWasm: WASM,
      transaction: shieldedTransaction,
      pspTransaction: pspTransactionInput,
      account: buyerUser.account,
      root,
    });

    const systemProof = await getSystemProof({
      account: buyerUser.account,
      systemProofInputs: proofInputs,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      inputUtxos,
    });
    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const completePspProofInputs = setUndefinedPspCircuitInputsToZero(
      proofInputs,
      IDL,
      pspTransactionInput.circuitName,
    );

    const pspProof = await buyerUser.account.getProofInternal({
      firstPath: pspTransactionInput.path,
      verifierIdl: pspTransactionInput.verifierIdl,
      proofInput: completePspProofInputs,
      inputUtxos,
    });
    /**
     * Create solana transactions.
     * We send 3 transactions because it is too much data for one solana transaction (max 1232 bytes).
     * Data:
     * - systemProof: 256 bytes,
     * - pspProof: 256 bytes,
     * - systemProofPublicInputs:
     * -
     */
    const solanaTransactionInputs: SolanaTransactionInputs = {
      action: Action.TRANSFER,
      systemProof,
      pspProof,
      publicTransactionVariables: shieldedTransaction.public,
      pspTransactionInput,
      rpcRecipientSol: RPC.accounts.rpcRecipientSol,
      eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      rootIndex,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: buyerUser.provider,
    });
    console.log("tx Hash : ", res.txHash);

    await buyerUser.getBalance();
    // check that the utxos are part of the users balance now
    assert(buyerUser.getUtxo(changeUtxo.utxoHash) !== undefined);
    assert(buyerUser.getUtxo(tradeOutputUtxo.utxoHash) !== undefined);
    let sellerUtxoInbox = await sellerUser.getUtxoInbox();
    console.log("seller utxo inbox ", sellerUtxoInbox);
    assert.equal(
      sellerUtxoInbox.totalSolBalance.toNumber(),
      offerUtxo.outUtxo.utxoData.priceSol.toNumber(),
    );
  });

  it("Swap Counter Offer functional", async () => {
    /**
     * 1. Create seller and buyer Users
     * 2. seller user creates offer
     *    - creates utxo
     *    - encrypts it to the buyer
     *    - stores the encrypted utxo on-chain in a compressed account
     * 3. recipient decrypts offer
     * 4. recipient generates counter-offer
     *    - creates utxo
     *    - encrypts it to the seller
     *    - stores the encrypted utxo on-chain in a compressed account
     * 5. seller decrypts counter-offer
     * 6. seller generates swap proof and settles the swap
     */
    const sellerUser: User = await createTestUser(provider.connection, 10e9);
    const buyerUser: User = await createTestUser(provider.connection, 110e9, 5);
    console.log(
      "new BN(sellerUser.account.encryptionKeypair.publicKey) ",
      new BN(sellerUser.account.encryptionKeypair.publicKey),
    );
    // TODO: add sorting to compute utxo data hash consistently
    // TODO: remove include appdata
    let offerUtxo = createProgramOutUtxo({
      lightWasm: WASM,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: buyerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e9)],
      utxoData: {
        priceSol: new BN(2e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: sellerUser.account.keypair.publicKey,
        recipientEncryptionPublicKey: hashAndTruncateToCircuit(
          sellerUser.account.encryptionKeypair.publicKey,
        ),
        // blinding: new BN(0),
      },
      pspIdl: IDL,
      pspId: verifierProgramId,
      utxoName: "utxo",
    });
    let txHashMakeOffer = await shieldProgramUtxo({
      account: sellerUser.account,
      appUtxo: offerUtxo,
      provider: sellerUser.provider,
    });
    console.log("made offer: ", txHashMakeOffer);

    let syncedStorage = await buyerUser.syncStorage(IDL, false);

    //TODO: refactor to only have one program utxo layer then an utxo array
    let fetchedOfferUtxo: Utxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];

    assert.deepEqual(
      JSON.stringify(offerUtxo.outUtxo.utxoData),
      JSON.stringify(fetchedOfferUtxo.utxoData),
    );
    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.utxoData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.utxoData.recipient.toString()}`,
    );
    /**
     * Offer trade 1 sol for 2 sol
     * Counter offer trade 1 sol for 1.5 sol
     */

    let counterOfferUtxo = createProgramOutUtxo({
      lightWasm: WASM,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: sellerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(15e8)],
      utxoData: {
        priceSol: new BN(1e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: buyerUser.account.keypair.publicKey,
        recipientEncryptionPublicKey: hashAndTruncateToCircuit(
          buyerUser.account.encryptionKeypair.publicKey,
        ),
      },
      pspIdl: IDL,
      pspId: verifierProgramId,
      utxoName: "utxo",
    });

    let txHashMakeCounterOffer = await shieldProgramUtxo({
      account: buyerUser.account,
      appUtxo: counterOfferUtxo,
      provider: buyerUser.provider,
    });
    console.log("made counter offer: ", txHashMakeCounterOffer);

    let syncedSellerStorage = await sellerUser.syncStorage(IDL, false);
    //TODO: refactor to only have one program utxo layer then an utxo array
    let fetchedCounterOfferUtxo: Utxo = Array.from(
      syncedSellerStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];

    assert.deepEqual(
      JSON.stringify(counterOfferUtxo.outUtxo.utxoData),
      JSON.stringify(fetchedCounterOfferUtxo.utxoData),
    );
    console.log(
      `Successfully fetched and decrypted counter offer: priceSol ${fetchedCounterOfferUtxo.utxoData.priceSol.toString()}, offer sol amount: ${fetchedCounterOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedCounterOfferUtxo.utxoData.recipient.toString()}`,
    );

    const circuitPath = path.join("build-circuit/swaps/swaps");

    // TODO: throw error if the pubkey is not mine and there is no encryption key specified
    const counterOfferRewardUtxo = createOutUtxo({
      lightWasm: WASM,
      publicKey: fetchedCounterOfferUtxo.utxoData.recipient,
      encryptionPublicKey: buyerUser.account.encryptionKeypair.publicKey,
      // TODO: Make this utxo works with:
      //     Uint8Array.from(
      //   fetchedCounterOfferUtxo.utxoData.recipientEncryptionPublicKey.toArray(),
      // ),
      amounts: [offerUtxo.outUtxo.amounts[0]],
      assets: [SystemProgram.programId],
      blinding: new BN(fetchedCounterOfferUtxo.blinding),
    });
    console.log(
      "fetchedOfferUtxo blinding: ",
      fetchedCounterOfferUtxo.blinding.toString(),
    );
    console.log(
      "offerRewardUtxo blinding: ",
      counterOfferRewardUtxo.blinding.toString(),
    );
    const tradeOutputUtxo = createOutUtxo({
      lightWasm: WASM,
      publicKey: sellerUser.account.keypair.publicKey,
      amounts: [
        fetchedCounterOfferUtxo.amounts[0].sub(sellerUser.provider.rpc.rpcFee),
      ],
      assets: [SystemProgram.programId],
    });

    // should I bundle it here or go through this step by step?
    // TODO: abstraction that unifies Transaction creation
    const pspTransactionInput: PspTransactionInput = {
      proofInputs: {
        takeCounterOfferInstruction: new BN(1),
      },
      path: circuitPath,
      verifierIdl: IDL,
      circuitName: "swaps",
      checkedInUtxos: [
        { utxoName: "offerUtxo", utxo: fetchedOfferUtxo },
        { utxoName: "counterOfferUtxo", utxo: fetchedCounterOfferUtxo },
      ],
      checkedOutUtxos: [
        { utxoName: "counterOfferRewardUtxo", utxo: counterOfferRewardUtxo },
      ],
      outUtxos: [tradeOutputUtxo],
    };
    const {
      syncedUtxos: inputUtxos,
      root,
      index: rootIndex,
    } = await syncInputUtxosMerkleProofs({
      inputUtxos: [fetchedOfferUtxo, fetchedCounterOfferUtxo],
      rpc: RPC,
      merkleTreePublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });
    const outputUtxos = [tradeOutputUtxo, counterOfferRewardUtxo];

    const shieldedTransaction = await createTransaction({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      rpcPublicKey: RPC.accounts.rpcPubkey,
      lightWasm: WASM,
      rpcFee: RPC.rpcFee,
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: sellerUser.account,
    });
    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const proofInputs = createProofInputs({
      lightWasm: WASM,
      transaction: shieldedTransaction,
      pspTransaction: pspTransactionInput,
      account: sellerUser.account,
      root,
    });

    const systemProof = await getSystemProof({
      account: sellerUser.account,
      systemProofInputs: proofInputs,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      inputUtxos,
    });
    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const completePspProofInputs = setUndefinedPspCircuitInputsToZero(
      proofInputs,
      IDL,
      pspTransactionInput.circuitName,
    );

    const pspProof = await sellerUser.account.getProofInternal({
      firstPath: pspTransactionInput.path,
      verifierIdl: pspTransactionInput.verifierIdl,
      proofInput: completePspProofInputs,
      inputUtxos,
    });
    /**
     * Create solana transactions.
     * We send 3 transactions because it is too much data for one solana transaction (max 1232 bytes).
     * Data:
     * - systemProof: 256 bytes,
     * - pspProof: 256 bytes,
     * - systemProofPublicInputs:
     * -
     */

    const solanaTransactionInputs: SolanaTransactionInputs = {
      action: Action.TRANSFER,
      systemProof,
      pspProof,
      publicTransactionVariables: shieldedTransaction.public,
      pspTransactionInput,
      rpcRecipientSol: RPC.accounts.rpcRecipientSol,
      eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      rootIndex,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: sellerUser.provider,
    });
    console.log("tx Hash : ", res.txHash);

    let sellerBalance = await sellerUser.getBalance();
    // check that the utxos are part of the users balance now
    assert(sellerUser.getUtxo(tradeOutputUtxo.utxoHash) !== undefined);
    assert.equal(
      sellerBalance.totalSolBalance.toNumber(),
      counterOfferUtxo.outUtxo.amounts[0]
        .sub(sellerUser.provider.rpc.rpcFee)
        .toNumber(),
    );

    let buyerUtxoInbox = await buyerUser.getUtxoInbox();
    console.log("buyer utxo inbox ", buyerUtxoInbox);
    assert.equal(
      buyerUtxoInbox.totalSolBalance.toNumber(),
      offerUtxo.outUtxo.amounts[0].toNumber(),
    );
  });

  it("Swap Cancel functional", async () => {
    /**
     * 1. Create seller and buyer Users
     * 2. seller user creates offer
     *    - creates utxo
     *    - encrypts it to herself (since this is just a test)
     *    - stores the encrypted utxo onchain in a compressed account
     * 3. seller generates cancel proof
     * 4. seller cancels the offer
     */
    const sellerUser: User = await createTestUser(provider.connection, 10e9);
    console.log(
      "new BN(sellerUser.account.encryptionKeypair.publicKey) ",
      new BN(sellerUser.account.encryptionKeypair.publicKey),
    );
    // TODO: add sorting to compute utxo data hash consistently
    // TODO: remove include appdata
    let offerUtxo = createProgramOutUtxo({
      lightWasm: WASM,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: sellerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e9)],
      utxoData: {
        priceSol: new BN(2e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: sellerUser.account.keypair.publicKey,
        recipientEncryptionPublicKey: hashAndTruncateToCircuit(
          sellerUser.account.encryptionKeypair.publicKey,
        ),
      },
      pspIdl: IDL,
      pspId: verifierProgramId,
      utxoName: "utxo",
    });

    let txHashMakeOffer = await shieldProgramUtxo({
      account: sellerUser.account,
      appUtxo: offerUtxo,
      provider: sellerUser.provider,
    });
    console.log("made offer: ", txHashMakeOffer);

    let syncedStorage = await sellerUser.syncStorage(IDL, false);

    //TODO: refactor to only have one program utxo layer then an utxo array
    let fetchedOfferUtxo: Utxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];

    assert.deepEqual(
      JSON.stringify(offerUtxo.outUtxo.utxoData),
      JSON.stringify(fetchedOfferUtxo.utxoData),
    );
    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.utxoData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.utxoData.recipient.toString()}`,
    );
    const circuitPath = path.join("build-circuit/swaps/swaps");

    const cancelOutputUtxo = createOutUtxo({
      lightWasm: WASM,
      publicKey: fetchedOfferUtxo.utxoData.recipient,
      amounts: [
        offerUtxo.outUtxo.amounts[0].sub(sellerUser.provider.rpc.rpcFee),
      ],
      assets: [SystemProgram.programId],
    });

    const emptySignerUtxo = createFillingUtxo({
      lightWasm: WASM,
      account: sellerUser.account,
    });

    // should I bundle it here or go through this step by step?
    // TODO: abstraction that unifies Transaction creation
    const pspTransactionInput: PspTransactionInput = {
      proofInputs: {
        cancelInstruction: new BN(1),
      },
      path: circuitPath,
      verifierIdl: IDL,
      circuitName: "swaps",
      checkedInUtxos: [
        { utxoName: "offerUtxo", utxo: fetchedOfferUtxo },
        { utxoName: "cancelSignerUtxo", utxo: emptySignerUtxo },
      ],
      outUtxos: [cancelOutputUtxo],
    };

    const {
      syncedUtxos: inputUtxos,
      root,
      index: rootIndex,
    } = await syncInputUtxosMerkleProofs({
      inputUtxos: [fetchedOfferUtxo, emptySignerUtxo],
      rpc: RPC,
      merkleTreePublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    });

    const outputUtxos = [cancelOutputUtxo];

    const shieldedTransaction = await createTransaction({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      rpcPublicKey: RPC.accounts.rpcPubkey,
      lightWasm: WASM,
      rpcFee: RPC.rpcFee,
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: sellerUser.account,
    });
    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const proofInputs = createProofInputs({
      lightWasm: WASM,
      transaction: shieldedTransaction,
      pspTransaction: pspTransactionInput,
      account: sellerUser.account,
      root,
    });

    const systemProof = await getSystemProof({
      account: sellerUser.account,
      systemProofInputs: proofInputs,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      inputUtxos,
    });

    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const completePspProofInputs = setUndefinedPspCircuitInputsToZero(
      proofInputs,
      IDL,
      pspTransactionInput.circuitName,
    );

    const pspProof = await sellerUser.account.getProofInternal({
      firstPath: pspTransactionInput.path,
      verifierIdl: pspTransactionInput.verifierIdl,
      proofInput: completePspProofInputs,
      inputUtxos,
    });
    /**
     * Create solana transactions.
     * We send 3 transactions because it is too much data for one solana transaction (max 1232 bytes).
     * Data:
     * - systemProof: 256 bytes,
     * - pspProof: 256 bytes,
     * - systemProofPublicInputs:
     * -
     */

    const solanaTransactionInputs: SolanaTransactionInputs = {
      action: Action.TRANSFER,
      systemProof,
      pspProof,
      publicTransactionVariables: shieldedTransaction.public,
      pspTransactionInput,
      rpcRecipientSol: RPC.accounts.rpcRecipientSol,
      eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      rootIndex,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: sellerUser.provider,
    });
    console.log("tx Hash : ", res.txHash);

    const balance = await sellerUser.getBalance();
    // check that the utxos are part of the users balance now
    assert(sellerUser.getUtxo(cancelOutputUtxo.utxoHash) !== undefined);
    assert.equal(
      balance.totalSolBalance.toNumber(),
      cancelOutputUtxo.amounts[0].toNumber(),
    );
  });
});
