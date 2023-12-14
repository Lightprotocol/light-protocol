import * as anchor from "@coral-xyz/anchor";
import {
  Utxo,
  Provider as LightProvider,
  confirmConfig,
  Action,
  TestRelayer,
  User,
  airdropSol,
  STANDARD_SHIELDED_PUBLIC_KEY,
  BN_0,
  PspTransactionInput,
  TransactionParameters,
  MerkleTreeConfig,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  getVerifierStatePda,
  createProofInputs,
  getSystemProof,
  setUndefinedPspCircuitInputsToZero,
  SolanaTransactionInputs,
  Provider,
  sendAndConfirmShieldedTransaction,
  ConfirmOptions,
  hashAndTruncateToCircuit,
  createTransaction,
  lightPsp4in4outAppStorageId,
} from "@lightprotocol/zk.js";

import { SystemProgram, PublicKey, Keypair, Connection } from "@solana/web3.js";
import { Hasher, WasmHasher } from "@lightprotocol/account.rs";
import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/swaps";
const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
);
import { assert } from "chai";

let HASHER: Hasher, RELAYER: TestRelayer;
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
    relayer: RELAYER,
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
    HASHER = await WasmHasher.getInstance();
    const relayerWallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e11,
      recipientPublicKey: relayerWallet.publicKey,
    });
    RELAYER = new TestRelayer({
      relayerPubkey: relayerWallet.publicKey,
      relayerRecipientSol: relayerWallet.publicKey,
      relayerFee: new BN(100000),
      payer: relayerWallet,
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
    let offerUtxo = new Utxo({
      hasher: HASHER,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: buyerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e9)],
      appData: {
        priceSol: new BN(2e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: sellerUser.account.pubkey,
        recipientEncryptionPublicKey: hashAndTruncateToCircuit(
          sellerUser.account.encryptionKeypair.publicKey,
        ),
        // blinding: new BN(0),
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
    });

    let txHashMakeOffer = await sellerUser.storeAppUtxo({
      appUtxo: offerUtxo,
      action: Action.SHIELD,
    });
    console.log("made offer: ", txHashMakeOffer);

    let syncedStorage = await buyerUser.syncStorage(IDL, false);
    await buyerUser.provider.latestMerkleTree();
    //TODO: refactor to only have one program utxo layer then an utxo array
    let fetchedOfferUtxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];
    // TODO: I need a standard public key flag
    fetchedOfferUtxo.publicKey = STANDARD_SHIELDED_PUBLIC_KEY;
    offerUtxo.index = fetchedOfferUtxo.index;
    Utxo.equal(HASHER, offerUtxo, fetchedOfferUtxo); // , false, sellerUser.account, buyerUser.account

    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.appData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.appData.recipient.toString()}`,
    );
    const circuitPath = path.join("build-circuit/swaps/swaps");

    const shieldUtxo = buyerUser.getAllUtxos()[0];

    // TODO: throw error if the pubkey is not mine and there is no encryption key specified
    const offerRewardUtxo = new Utxo({
      hasher: HASHER,
      publicKey: fetchedOfferUtxo.appData.recipient,
      encryptionPublicKey: sellerUser.account.encryptionKeypair.publicKey,
      // TODO: Make this utxo works with:
      // Uint8Array.from(
      //   fetchedOfferUtxo.appData.recipientEncryptionPublicKey.toArray(),
      // ),
      assetLookupTable: buyerUser.provider.lookUpTables.assetLookupTable,
      amounts: [new BN(2e9)],
      assets: [SystemProgram.programId],
      blinding: fetchedOfferUtxo.blinding,
    });
    console.log(
      "fetchedOfferUtxo blinding: ",
      fetchedOfferUtxo.blinding.toString(),
    );
    console.log(
      "offerRewardUtxo blinding: ",
      offerRewardUtxo.blinding.toString(),
    );
    const tradeOutputUtxo = new Utxo({
      hasher: HASHER,
      publicKey: fetchedOfferUtxo.appData.recipient,
      assetLookupTable: buyerUser.provider.lookUpTables.assetLookupTable,
      amounts: [new BN(1e9)],
      assets: [SystemProgram.programId],
    });

    const changeAmountSol = shieldUtxo.amounts[0]
      .sub(offerRewardUtxo.amounts[0])
      .sub(RELAYER.relayerFee);

    // TODO: add function to create change utxo
    const changeUtxo = new Utxo({
      hasher: HASHER,
      publicKey: fetchedOfferUtxo.appData.recipient,
      assetLookupTable: buyerUser.provider.lookUpTables.assetLookupTable,
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

    const inputUtxos = [fetchedOfferUtxo, shieldUtxo];
    const outputUtxos = [changeUtxo, tradeOutputUtxo, offerRewardUtxo];
    const shieldedTransaction = await createTransaction({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      relayerPublicKey: RELAYER.accounts.relayerPubkey,
      hasher: HASHER,
      relayerFee: RELAYER.relayerFee,
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: buyerUser.account,
      root: buyerUser.provider.solMerkleTree.merkleTree.root(),
    });

    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const proofInputs = createProofInputs({
      hasher: HASHER,
      transaction: shieldedTransaction,
      pspTransaction: pspTransactionInput,
      account: buyerUser.account,
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
      relayerRecipientSol: RELAYER.accounts.relayerRecipientSol,
      eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: buyerUser.provider,
    });
    console.log("tx Hash : ", res.txHash);

    await buyerUser.getBalance();
    // check that the utxos are part of the users balance now
    assert(buyerUser.getUtxo(changeUtxo.getCommitment(HASHER)) !== undefined);
    assert(
      buyerUser.getUtxo(tradeOutputUtxo.getCommitment(HASHER)) !== undefined,
    );
    let sellerUtxoInbox = await sellerUser.getUtxoInbox();
    console.log("seller utxo inbox ", sellerUtxoInbox);
    assert.equal(
      sellerUtxoInbox.totalSolBalance.toNumber(),
      offerUtxo.appData.priceSol.toNumber(),
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
    let offerUtxo = new Utxo({
      hasher: HASHER,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: buyerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e9)],
      appData: {
        priceSol: new BN(2e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: sellerUser.account.pubkey,
        recipientEncryptionPublicKey: hashAndTruncateToCircuit(
          sellerUser.account.encryptionKeypair.publicKey,
        ),
        // blinding: new BN(0),
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
    });

    let txHashMakeOffer = await sellerUser.storeAppUtxo({
      appUtxo: offerUtxo,
      action: Action.SHIELD,
    });
    console.log("made offer: ", txHashMakeOffer);

    let syncedStorage = await buyerUser.syncStorage(IDL, false);
    await buyerUser.provider.latestMerkleTree();
    //TODO: refactor to only have one program utxo layer then an utxo array
    let fetchedOfferUtxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];
    // TODO: I need a standard public key flag
    fetchedOfferUtxo.publicKey = STANDARD_SHIELDED_PUBLIC_KEY;
    offerUtxo.index = fetchedOfferUtxo.index;
    Utxo.equal(HASHER, offerUtxo, fetchedOfferUtxo); // , false, sellerUser.account, buyerUser.account

    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.appData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.appData.recipient.toString()}`,
    );
    /**
     * Offer trade 1 sol for 2 sol
     * Counter offer trade 1 sol for 1.5 sol
     */

    let counterOfferUtxo = new Utxo({
      hasher: HASHER,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: sellerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(15e8)],
      appData: {
        priceSol: new BN(1e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: buyerUser.account.pubkey,
        recipientEncryptionPublicKey: hashAndTruncateToCircuit(
          buyerUser.account.encryptionKeypair.publicKey,
        ),
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
    });

    let txHashMakeCounterOffer = await buyerUser.storeAppUtxo({
      appUtxo: counterOfferUtxo,
      action: Action.TRANSFER,
    });
    console.log("made counter offer: ", txHashMakeCounterOffer);

    let syncedSellerStorage = await sellerUser.syncStorage(IDL, false);
    await sellerUser.provider.latestMerkleTree();
    //TODO: refactor to only have one program utxo layer then an utxo array
    let fetchedCounterOfferUtxo = Array.from(
      syncedSellerStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];
    // TODO: I need a standard public key flag
    fetchedCounterOfferUtxo.publicKey = STANDARD_SHIELDED_PUBLIC_KEY;
    counterOfferUtxo.index = fetchedCounterOfferUtxo.index;
    Utxo.equal(HASHER, counterOfferUtxo, fetchedCounterOfferUtxo);
    console.log(
      `Successfully fetched and decrypted counter offer: priceSol ${fetchedCounterOfferUtxo.appData.priceSol.toString()}, offer sol amount: ${fetchedCounterOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedCounterOfferUtxo.appData.recipient.toString()}`,
    );

    const circuitPath = path.join("build-circuit/swaps/swaps");

    // const shieldUtxo = sellerUser.getAllUtxos()[0];

    // TODO: throw error if the pubkey is not mine and there is no encryption key specified
    const counterOfferRewardUtxo = new Utxo({
      hasher: HASHER,
      publicKey: fetchedCounterOfferUtxo.appData.recipient,
      encryptionPublicKey: buyerUser.account.encryptionKeypair.publicKey,
      // TODO: Make this utxo works with:
      //     Uint8Array.from(
      //   fetchedCounterOfferUtxo.appData.recipientEncryptionPublicKey.toArray(),
      // ),
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
      amounts: [offerUtxo.amounts[0]],
      assets: [SystemProgram.programId],
      blinding: fetchedCounterOfferUtxo.blinding,
    });
    console.log(
      "fetchedOfferUtxo blinding: ",
      fetchedCounterOfferUtxo.blinding.toString(),
    );
    console.log(
      "offerRewardUtxo blinding: ",
      counterOfferRewardUtxo.blinding.toString(),
    );
    const tradeOutputUtxo = new Utxo({
      hasher: HASHER,
      publicKey: sellerUser.account.pubkey,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
      amounts: [
        fetchedCounterOfferUtxo.amounts[0].sub(
          sellerUser.provider.relayer.relayerFee,
        ),
      ],
      assets: [SystemProgram.programId],
    });

    // const changeAmountSol = tradeOutputUtxo.amounts[0]
    //   .sub(counterOfferRewardUtxo.amounts[0])
    //   .sub(RELAYER.relayerFee);

    // // TODO: add function to create change utxo
    // const changeUtxo = new Utxo({
    //   poseidon: POSEIDON,
    //   publicKey: fetchedOfferUtxo.appData.recipient,
    //   assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
    //   verifierProgramLookupTable:
    //     sellerUser.provider.lookUpTables.verifierProgramLookupTable,
    //   amounts: [changeAmountSol],
    //   assets: [SystemProgram.programId],
    // });

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
        { utxoName: "offerUtxo", utxo: offerUtxo },
        { utxoName: "counterOfferUtxo", utxo: fetchedCounterOfferUtxo },
      ],
      checkedOutUtxos: [
        { utxoName: "counterOfferRewardUtxo", utxo: counterOfferRewardUtxo },
      ],
      outUtxos: [tradeOutputUtxo],
    };

    const inputUtxos = [offerUtxo, fetchedCounterOfferUtxo];
    const outputUtxos = [tradeOutputUtxo, counterOfferRewardUtxo];

    const shieldedTransaction = await createTransaction({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      relayerPublicKey: RELAYER.accounts.relayerPubkey,
      hasher: HASHER,
      relayerFee: RELAYER.relayerFee,
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: sellerUser.account,
      root: sellerUser.provider.solMerkleTree.merkleTree.root(),
    });
    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const proofInputs = createProofInputs({
      hasher: HASHER,
      transaction: shieldedTransaction,
      pspTransaction: pspTransactionInput,
      account: sellerUser.account,
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
      relayerRecipientSol: RELAYER.accounts.relayerRecipientSol,
      eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: sellerUser.provider,
    });
    console.log("tx Hash : ", res.txHash);

    let sellerBalance = await sellerUser.getBalance();
    // check that the utxos are part of the users balance now
    assert(
      sellerUser.getUtxo(tradeOutputUtxo.getCommitment(HASHER)) !== undefined,
    );
    assert.equal(
      sellerBalance.totalSolBalance.toNumber(),
      counterOfferUtxo.amounts[0]
        .sub(sellerUser.provider.relayer.relayerFee)
        .toNumber(),
    );

    let buyerUtxoInbox = await buyerUser.getUtxoInbox();
    console.log("buyer utxo inbox ", buyerUtxoInbox);
    assert.equal(
      buyerUtxoInbox.totalSolBalance.toNumber(),
      offerUtxo.amounts[0].toNumber(),
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
    let offerUtxo = new Utxo({
      hasher: HASHER,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: sellerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e9)],
      appData: {
        priceSol: new BN(2e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: sellerUser.account.pubkey,
        recipientEncryptionPublicKey: hashAndTruncateToCircuit(
          sellerUser.account.encryptionKeypair.publicKey,
        ),
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
    });

    let txHashMakeOffer = await sellerUser.storeAppUtxo({
      appUtxo: offerUtxo,
      action: Action.SHIELD,
    });
    console.log("made offer: ", txHashMakeOffer);

    let syncedStorage = await sellerUser.syncStorage(IDL, false);
    await sellerUser.provider.latestMerkleTree();
    //TODO: refactor to only have one program utxo layer then an utxo array
    let fetchedOfferUtxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];
    // TODO: I need a standard public key flag
    fetchedOfferUtxo.publicKey = STANDARD_SHIELDED_PUBLIC_KEY;
    offerUtxo.index = fetchedOfferUtxo.index;
    Utxo.equal(HASHER, offerUtxo, fetchedOfferUtxo); // , false, sellerUser.account, buyerUser.account

    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.appData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.appData.recipient.toString()}`,
    );
    const circuitPath = path.join("build-circuit/swaps/swaps");

    const cancelOutputUtxo = new Utxo({
      hasher: HASHER,
      publicKey: fetchedOfferUtxo.appData.recipient,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
      amounts: [
        offerUtxo.amounts[0].sub(sellerUser.provider.relayer.relayerFee),
      ],
      assets: [SystemProgram.programId],
    });

    const emptySignerUtxo = new Utxo({
      hasher: HASHER,
      publicKey: sellerUser.account.pubkey,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
      amounts: [BN_0],
      assets: [SystemProgram.programId],
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

    const inputUtxos = [fetchedOfferUtxo, emptySignerUtxo];
    const outputUtxos = [cancelOutputUtxo];

    const shieldedTransaction = await createTransaction({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      relayerPublicKey: RELAYER.accounts.relayerPubkey,
      hasher: HASHER,
      relayerFee: RELAYER.relayerFee,
      pspId: verifierProgramId,
      systemPspId: lightPsp4in4outAppStorageId,
      account: sellerUser.account,
      root: sellerUser.provider.solMerkleTree.merkleTree.root(),
    });
    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const proofInputs = createProofInputs({
      hasher: HASHER,
      transaction: shieldedTransaction,
      pspTransaction: pspTransactionInput,
      account: sellerUser.account,
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
      relayerRecipientSol: RELAYER.accounts.relayerRecipientSol,
      eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
      systemPspIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: sellerUser.provider,
    });
    console.log("tx Hash : ", res.txHash);

    const balance = await sellerUser.getBalance();
    // check that the utxos are part of the users balance now
    assert(
      sellerUser.getUtxo(cancelOutputUtxo.getCommitment(HASHER)) !== undefined,
    );
    assert.equal(
      balance.totalSolBalance.toNumber(),
      cancelOutputUtxo.amounts[0].toNumber(),
    );
  });
});
