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
  airdropSplToAssociatedTokenAccount,
  MINT,
  hashAndTruncateToCircuit,
  createTestAccounts,
} from "@lightprotocol/zk.js";

import { SystemProgram, PublicKey, Keypair, Connection } from "@solana/web3.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/swaps";
const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
);
import { assert } from "chai";

let POSEIDON: any, RELAYER: TestRelayer;
const RPC_URL = "http://127.0.0.1:8899";

/**
 * Creates a test user with airdropped lamports.
 * @param connection
 * @param lamports
 * @param shieldedSol
 * @returns
 */
const createTestUser = async (
  connection: Connection,
  lamports: number,
  shieldedSol?: number,
): Promise<User> => {
  let wallet = Keypair.generate();
  await airdropSol({
    connection,
    lamports,
    recipientPublicKey: wallet.publicKey,
  });

  const lightProvider: Provider = await LightProvider.init({
    wallet,
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
    POSEIDON = await buildPoseidonOpt();
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
    // Creates test accounts, among others the token MINT
    await createTestAccounts(provider.connection);
  });

  /**
   * 1. Create seller and buyer Users
   * 2. seller user creates offer
   *    - creates utxo
   *    - encrypts it to the buyer
   *    - stores the encrypted utxo onchain in a compressed account
   * 3. recipient decrypts offer
   * 4. recipient generates
   */
  it.only("Swap Take functional", async () => {
    /**
     * Step 1: Create and fund seller and buyer users.
     * ---------------------------------------------------
     */

    /**
     * Creates the seller and buyer users with random keypairs and 100 airdropped sol.
     */
    const sellerUser: User = await createTestUser(provider.connection, 100e9);
    const buyerUser: User = await createTestUser(provider.connection, 100e9);

    /**
     * Seller shields 10 sol to fund her shielded account.
     */
    await sellerUser.shield({
      token: "SOL",
      publicAmountSol: 10,
      confirmOptions: ConfirmOptions.finalized,
    });

    /**
     * Airdrop 400 usdc to the buyer.
     */
    const usdcDecimals = 1e2;
    await airdropSplToAssociatedTokenAccount(
      provider.connection,
      400 * usdcDecimals,
      buyerUser.account.solanaPublicKey!,
    );

    /**
     * Buyer shields 6 sol and 400 USDC to fund his shielded account.
     */
    await buyerUser.shield({
      token: "USDC",
      publicAmountSpl: 400,
      publicAmountSol: 6,
    });

    /**
     * Step 2: Create offer utxo, to swap 1 sol for 2 sol.
     * ---------------------------------------------------
     */

    /**
     * Create offer utxo, to swap 10 sol for 300 USDC.
     * The amount to be traded is store in the utxos amounts field.
     * The utxo data determines the trade parameters:
     * - priceSol: 0 sol (Is zero since price is in USDC)
     * - priceSpl: 300
     * - splAsset: USDC (the hashed and truncated mint address, so that it is smaller than the circuit field size)
     * - recipient: is the maker (seller user public key)
     */
    let offerUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: buyerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e10)],
      appData: {
        priceSol: new BN(0),
        priceSpl: new BN(400 * usdcDecimals),
        splAsset: hashAndTruncateToCircuit(MINT.toBytes()),
        recipient: sellerUser.account.pubkey,
        recipientEncryptionPublicKey: new BN(
          sellerUser.account.encryptionKeypair.publicKey,
        ),
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
    });

    /**
     * Insert the offer utxo into Light Protocol state.
     * Store the encrypted offer utxo onchain into compressed account.
     */
    let txHashMakeOffer = await sellerUser.storeAppUtxo({
      appUtxo: offerUtxo,
      action: Action.SHIELD,
    });
    console.log("Made encrypted offer signature: ", txHashMakeOffer);

    /**
     * Step 3: Buyer fetch and decrypt offer.
     * ---------------------------------------------------
     */

    /**
     * Fetch the encrypted offer utxo from the compressed account.
     * Decrypt the offer utxo.
     * syncStorage syncs
     */
    let syncedStorage = await buyerUser.syncStorage(IDL, false);
    await buyerUser.provider.latestMerkleTree();

    let fetchedOfferUtxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];

    fetchedOfferUtxo.publicKey = STANDARD_SHIELDED_PUBLIC_KEY;
    offerUtxo.index = fetchedOfferUtxo.index;
    Utxo.equal(POSEIDON, offerUtxo, fetchedOfferUtxo);

    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.appData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.appData.recipient.toString()}\n`,
    );

    /**
     * Step 4: Create utxos.
     * ---------------------------------------------------
     */

    const offerRewardUtxo = new Utxo({
      poseidon: POSEIDON,
      publicKey: fetchedOfferUtxo.appData.recipient,
      encryptionPublicKey: Uint8Array.from(
        fetchedOfferUtxo.appData.recipientEncryptionPublicKey.toArray(),
      ),
      assetLookupTable: buyerUser.provider.lookUpTables.assetLookupTable,
      amounts: [new BN(0), offerUtxo.appData.priceSpl],
      assets: [SystemProgram.programId, MINT],
      blinding: fetchedOfferUtxo.blinding,
    });

    /**
     * tradeOutputUtxo is a native utxo which holds sol and is owned by the buyer.
     */
    const tradeOutputUtxo = new Utxo({
      poseidon: POSEIDON,
      publicKey: fetchedOfferUtxo.appData.recipient,
      assetLookupTable: buyerUser.provider.lookUpTables.assetLookupTable,
      amounts: [fetchedOfferUtxo.amounts[0]],
      assets: [SystemProgram.programId],
    });

    /**
     * feeUtxo is a native utxo which holds sol.
     * It is used to pay for the trade result the relayer fee.
     */
    const feeUtxo = buyerUser.getAllUtxos()[0];

    /**
     * changeUtxo is a native utxo which holds sol, and USDC.
     * It is used to return the change amounts to the buyer.
     * The change amounts are the difference between the offer amount and the trade amount.
     */
    const changeAmountSol = feeUtxo.amounts[0].sub(RELAYER.relayerFee);
    const changeAmountSpl = feeUtxo.amounts[1].sub(
      fetchedOfferUtxo.appData.priceSpl,
    );

    const changeUtxo = new Utxo({
      poseidon: POSEIDON,
      publicKey: fetchedOfferUtxo.appData.recipient,
      assetLookupTable: buyerUser.provider.lookUpTables.assetLookupTable,
      amounts: [changeAmountSol, changeAmountSpl],
      assets: [SystemProgram.programId, MINT],
    });

    /**
     * Path to the compiled circuit.
     * build-circuit is the default path.
     */
    const circuitPath = path.join("build-circuit");

    /**
     * pspTransactionInput bundles transaction inputs for the PSP transaction.
     * - we want to execute the takeOfferInstruction, thus we set:
     *   - takeOfferInstruction to 1
     *   - other proof inputs are either taken from the utxos defined
     *     or set as zero with setUndefinedPspCircuitInputsToZero
     *   - checkedInUtxos defines the offer utxo and
     * Input Utxos:
     * - the fee utxo adds the funds the buyer uses to pay for the trade
     * - the offer utxo is the utxo the buyer wants to take
     * Output Utxos:
     * - the trade output utxo holds the trade proceeds of the seller
     * - the change utxo holds the change amounts not required in the trade or to pay the relayer
     */
    const pspTransactionInput: PspTransactionInput = {
      proofInputs: {
        takeOfferInstruction: new BN(1),
      },
      path: circuitPath,
      verifierIdl: IDL,
      circuitName: "swaps",
      checkedInUtxos: [{ utxoName: "offerUtxo", utxo: fetchedOfferUtxo }],
      checkedOutUtxos: [{ utxoName: "offerRewardUtxo", utxo: offerRewardUtxo }],
      inUtxos: [feeUtxo],
      outUtxos: [changeUtxo, tradeOutputUtxo],
    };

    const inputUtxos = [fetchedOfferUtxo, feeUtxo];
    const outputUtxos = [changeUtxo, tradeOutputUtxo, offerRewardUtxo];

    const txParams = new TransactionParameters({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(new BN(0)),
      action: Action.TRANSFER,
      poseidon: POSEIDON,
      relayer: RELAYER,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      account: buyerUser.account,
      verifierState: getVerifierStatePda(
        verifierProgramId,
        RELAYER.accounts.relayerPubkey,
      ),
    });

    await txParams.getTxIntegrityHash(POSEIDON);

    /**
     * Creates the proof inputs for the PSP and system proofs of the PSP transaction.
     */
    const proofInputs = createProofInputs({
      poseidon: POSEIDON,
      transaction: txParams,
      pspTransaction: pspTransactionInput,
      account: buyerUser.account,
      solMerkleTree: buyerUser.provider.solMerkleTree,
    });

    /**
     * Generates the system proof.
     * The system proof proves the correct spending of input and creation of output utxos.
     * Input utxos have to exists in the protocol state.
     * Output utxos' asset amounts have to match the sums and assets of the input utxos.
     */
    const systemProof = await getSystemProof({
      account: buyerUser.account,
      transaction: txParams,
      systemProofInputs: proofInputs,
    });

    /**
     * Generates the PSP proof.
     * The PSP proof proves the PSP logic.
     * In this case it enforces the constraints that an offer utxo
     * can only be spent if a reward utxo exists for which the offer utxo data matches.
     */
    const completePspProofInputs = setUndefinedPspCircuitInputsToZero(
      proofInputs,
      IDL,
      pspTransactionInput.circuitName,
    );
    const pspProof = await buyerUser.account.getProofInternal(
      pspTransactionInput.path,
      pspTransactionInput,
      completePspProofInputs,
      false,
    );

    /**
     * Step 5:
     * Create solana transactions.
     * ---------------------------------------------------
     */

    /**
     * Create solana transactions.
     * We send 3 transactions because it is too much data for one solana transaction
     * (max 1232 bytes per solana tx).
     */
    const solanaTransactionInputs: SolanaTransactionInputs = {
      systemProof,
      pspProof,
      transaction: txParams,
      pspTransactionInput,
    };

    const shieldedTransactionConfirmation =
      await sendAndConfirmShieldedTransaction({
        solanaTransactionInputs,
        provider: buyerUser.provider,
        confirmOptions: ConfirmOptions.spendable,
      });
    console.log(
      "Take offer tx Hash : ",
      shieldedTransactionConfirmation.txHash,
    );

    /**
     * Get the balance of the buyer.
     * The balance should contain:
     * - trade output utxo
     * - change utxo.
     */
    await buyerUser.getBalance();
    assert(buyerUser.getUtxo(changeUtxo.getCommitment(POSEIDON)) !== undefined);
    assert(
      buyerUser.getUtxo(tradeOutputUtxo.getCommitment(POSEIDON)) !== undefined,
    );

    /**
     * Get the balance of the seller.
     * The inbox balance should contain:
     * - offer reward utxo
     *
     * We need to check the inbox balance because the utxo was encrypted asymetrically to the seller.
     * To make the utxo part of the spendable balance the seller needs to accept the utxo.
     */
    let sellerUtxoInbox = await sellerUser.getUtxoInbox();
    console.log("Seller utxo inbox ", sellerUtxoInbox);
    assert.equal(
      sellerUtxoInbox.totalSolBalance.toNumber(),
      offerUtxo.appData.priceSol.toNumber(),
    );
  });

  /**
   * 1. Create seller and buyer Users
   * 2. seller user creates offer
   *    - creates utxo
   *    - encrypts it to herself (since this is just a test)
   *    - stores the encrypted utxo onchain in a compressed account
   * 3. seller generates cancel proof
   * 4. seller cancels the offer
   */
  it("Swap Cancel functional", async () => {
    const sellerUser: User = await createTestUser(provider.connection, 10e9);

    let offerUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: sellerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e9)],
      appData: {
        priceSol: new BN(2e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: sellerUser.account.pubkey,
        recipientEncryptionPublicKey: new BN(
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

    let fetchedOfferUtxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];

    fetchedOfferUtxo.publicKey = STANDARD_SHIELDED_PUBLIC_KEY;
    offerUtxo.index = fetchedOfferUtxo.index;
    Utxo.equal(POSEIDON, offerUtxo, fetchedOfferUtxo);

    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.appData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.appData.recipient.toString()}`,
    );
    const circuitPath = path.join("build-circuit");

    const cancelOutputUtxo = new Utxo({
      poseidon: POSEIDON,
      publicKey: fetchedOfferUtxo.appData.recipient,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
      amounts: [
        offerUtxo.amounts[0].sub(sellerUser.provider.relayer.relayerFee),
      ],
      assets: [SystemProgram.programId],
    });

    const emptySignerUtxo = new Utxo({
      poseidon: POSEIDON,
      publicKey: sellerUser.account.pubkey,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
      amounts: [BN_0],
      assets: [SystemProgram.programId],
    });

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

    const txParams = new TransactionParameters({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(new BN(0)),
      action: Action.TRANSFER,
      poseidon: POSEIDON,
      relayer: RELAYER,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      account: sellerUser.account,
      verifierState: getVerifierStatePda(
        verifierProgramId,
        RELAYER.accounts.relayerPubkey,
      ),
    });

    await txParams.getTxIntegrityHash(POSEIDON);

    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const proofInputs = createProofInputs({
      poseidon: POSEIDON,
      transaction: txParams,
      pspTransaction: pspTransactionInput,
      account: sellerUser.account,
      solMerkleTree: sellerUser.provider.solMerkleTree,
    });

    const systemProof = await getSystemProof({
      account: sellerUser.account,
      transaction: txParams,
      systemProofInputs: proofInputs,
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
    const pspProof = await sellerUser.account.getProofInternal(
      pspTransactionInput.path,
      pspTransactionInput,
      completePspProofInputs,
      false,
    );
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
      systemProof,
      pspProof,
      transaction: txParams,
      pspTransactionInput,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: sellerUser.provider,
      confirmOptions: ConfirmOptions.spendable,
    });
    console.log("tx Hash : ", res.txHash);

    // check that the utxos are part of the users balance now
    const balance = await sellerUser.getBalance();
    assert(
      sellerUser.getUtxo(cancelOutputUtxo.getCommitment(POSEIDON)) !==
        undefined,
    );
    assert.equal(
      balance.totalSolBalance.toNumber(),
      cancelOutputUtxo.amounts[0].toNumber(),
    );
  });

  /**
   * 1. Create seller and buyer Users
   * 2. seller user creates offer
   *    - creates utxo
   *    - encrypts it to the buyer
   *    - stores the encrypted utxo onchain in a compressed account
   * 3. recipient decrypts offer
   * 4. recipient generates counter offer
   *    - creates utxo
   *    - encrypts it to the seller
   *    - stores the encrypted utxo onchain in a compressed account
   * 5. seller decrypts counter offer
   * 6. seller generates swap proof and settles the swap
   */
  it("Swap Counter Offer functional", async () => {
    const sellerUser: User = await createTestUser(provider.connection, 10e9);
    const buyerUser: User = await createTestUser(provider.connection, 110e9, 5);

    let offerUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: buyerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(1e9)],
      appData: {
        priceSol: new BN(2e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: sellerUser.account.pubkey,
        recipientEncryptionPublicKey: new BN(
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

    let syncedStorage = await buyerUser.syncStorage(IDL, false);
    await buyerUser.provider.latestMerkleTree();

    let fetchedOfferUtxo = Array.from(
      syncedStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];

    fetchedOfferUtxo.publicKey = STANDARD_SHIELDED_PUBLIC_KEY;
    offerUtxo.index = fetchedOfferUtxo.index;
    Utxo.equal(POSEIDON, offerUtxo, fetchedOfferUtxo);

    console.log(
      `Successfully fetched and decrypted offer: priceSol ${fetchedOfferUtxo.appData.priceSol.toString()}, offer sol amount: ${fetchedOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedOfferUtxo.appData.recipient.toString()}`,
    );
    /**
     * Offer trade 1 sol for 2 sol
     * Counter offer trade 1 sol for 1.5 sol
     */

    let counterOfferUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      encryptionPublicKey: sellerUser.account.encryptionKeypair.publicKey,
      amounts: [new BN(15e8)],
      appData: {
        priceSol: new BN(1e9),
        priceSpl: new BN(0),
        splAsset: new BN(0),
        recipient: buyerUser.account.pubkey,
        recipientEncryptionPublicKey: new BN(
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
    let fetchedCounterOfferUtxo = Array.from(
      syncedSellerStorage
        .get(verifierProgramId.toBase58())
        .tokenBalances.get(SystemProgram.programId.toBase58())
        .utxos.values(),
    )[0];
    fetchedCounterOfferUtxo.publicKey = STANDARD_SHIELDED_PUBLIC_KEY;
    counterOfferUtxo.index = fetchedCounterOfferUtxo.index;
    Utxo.equal(POSEIDON, counterOfferUtxo, fetchedCounterOfferUtxo);
    console.log(
      `Successfully fetched and decrypted counter offer: priceSol ${fetchedCounterOfferUtxo.appData.priceSol.toString()}, offer sol amount: ${fetchedCounterOfferUtxo.amounts[0].toString()} \n recipient public key: ${fetchedCounterOfferUtxo.appData.recipient.toString()}`,
    );

    const circuitPath = path.join("build-circuit");

    const counterOfferRewardUtxo = new Utxo({
      poseidon: POSEIDON,
      publicKey: fetchedCounterOfferUtxo.appData.recipient,
      encryptionPublicKey: Uint8Array.from(
        fetchedCounterOfferUtxo.appData.recipientEncryptionPublicKey.toArray(),
      ),
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
      poseidon: POSEIDON,
      publicKey: sellerUser.account.pubkey,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
      amounts: [
        fetchedCounterOfferUtxo.amounts[0].sub(
          sellerUser.provider.relayer.relayerFee,
        ),
      ],
      assets: [SystemProgram.programId],
    });

    const emptySignerUtxo = new Utxo({
      poseidon: POSEIDON,
      publicKey: sellerUser.account.pubkey,
      assetLookupTable: sellerUser.provider.lookUpTables.assetLookupTable,
      amounts: [BN_0],
      assets: [SystemProgram.programId],
    });

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
        { utxoName: "counterOfferSignerUtxo", utxo: emptySignerUtxo },
      ],
      checkedOutUtxos: [
        { utxoName: "counterOfferRewardUtxo", utxo: counterOfferRewardUtxo },
      ],
      outUtxos: [tradeOutputUtxo],
    };

    const inputUtxos = [offerUtxo, fetchedCounterOfferUtxo, emptySignerUtxo];
    const outputUtxos = [tradeOutputUtxo, counterOfferRewardUtxo];

    const txParams = new TransactionParameters({
      inputUtxos,
      outputUtxos,
      transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
        new BN(0),
      ),
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(new BN(0)),
      action: Action.TRANSFER,
      poseidon: POSEIDON,
      relayer: RELAYER,
      verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
      account: sellerUser.account,
      verifierState: getVerifierStatePda(
        verifierProgramId,
        RELAYER.accounts.relayerPubkey,
      ),
    });

    await txParams.getTxIntegrityHash(POSEIDON);

    /**
     * Proves PSP logic
     * returns proof and it's public inputs
     */

    const proofInputs = createProofInputs({
      poseidon: POSEIDON,
      transaction: txParams,
      pspTransaction: pspTransactionInput,
      account: sellerUser.account,
      solMerkleTree: sellerUser.provider.solMerkleTree,
    });

    const systemProof = await getSystemProof({
      account: sellerUser.account,
      transaction: txParams,
      systemProofInputs: proofInputs,
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
    const pspProof = await sellerUser.account.getProofInternal(
      pspTransactionInput.path,
      pspTransactionInput,
      completePspProofInputs,
      false,
    );
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
      systemProof,
      pspProof,
      transaction: txParams,
      pspTransactionInput,
    };

    const res = await sendAndConfirmShieldedTransaction({
      solanaTransactionInputs,
      provider: sellerUser.provider,
      confirmOptions: ConfirmOptions.spendable,
    });
    console.log("tx Hash : ", res.txHash);

    let sellerBalance = await sellerUser.getBalance();
    // check that the utxos are part of the users balance now
    assert(
      sellerUser.getUtxo(tradeOutputUtxo.getCommitment(POSEIDON)) !== undefined,
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
});
