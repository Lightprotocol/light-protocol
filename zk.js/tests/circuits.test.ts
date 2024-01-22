// TODO: refactor after circuit refactor
// import { assert } from "chai";
// const chai = require("chai");
// const chaiAsPromised = require("chai-as-promised");
// // Load chai-as-promised support
// chai.use(chaiAsPromised);
// import { Keypair as SolanaKeypair } from "@solana/web3.js";
// import { BN } from "@coral-xyz/anchor";
// import { it } from "mocha";

// import {
//   Account,
//   Utxo,
//   FEE_ASSET,
//   hashAndTruncateToCircuit,
//   Provider as LightProvider,
//   MINT,
//   Rpc,
//   TransactionErrorCode,
//   Action,
//   IDL_LIGHT_PSP2IN2OUT,
//   IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
//   BN_0,
//   BN_1,
// } from "../src";
// import { WasmFactory, Hasher } from "@lightprotocol/account.rs";
// import { IDL } from "./testData/tmp_test_psp";
// import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
// import { MerkleTree } from "@lightprotocol/circuit-lib.js";

// process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
// process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

// let account: Account,
//   compressUtxo1: Utxo,
//   mockPubkey,
//   hasher: Hasher,
//   lightProvider: LightProvider,
//   txParamsApp: TransactionParameters,
//   txParamsPoolType: TransactionParameters,
//   txParamsPoolTypeOut: TransactionParameters,
//   txParamsOutApp: TransactionParameters,
//   txParams: TransactionParameters,
//   txParamsSol: TransactionParameters,
//   paramsDecompress: TransactionParameters,
//   appData: any,
//   rpc: Rpc;
// const seed32 = bs58.encode(new Uint8Array(32).fill(1));

// // TODO: check more specific errors in tests
// describe("Masp circuit tests", () => {
//   before(async () => {
//     lightProvider = await LightProvider.loadMock();
//     hasher = await WasmFactory.getInstance();
//     account = new Account({ hasher, seed: seed32 });
//     await account.getEddsaPublicKey();
//     const compressAmount = 20_000;
//     const compressFeeAmount = 10_000;
//     compressUtxo1 = new Utxo({
//       index: 0,
//       hasher,
//       assets: [FEE_ASSET, MINT],
//       amounts: [new BN(compressFeeAmount), new BN(compressAmount)],
//       publicKey: account.pubkey,
//       assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//     });
//     const compressUtxoSol = new Utxo({
//       index: 0,
//       hasher,
//       assets: [FEE_ASSET, MINT],
//       amounts: [new BN(compressFeeAmount), BN_0],
//       publicKey: account.pubkey,
//       assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//     });
//     mockPubkey = SolanaKeypair.generate().publicKey;
//     const mockPubkey2 = SolanaKeypair.generate().publicKey;
//     const mockPubkey3 = SolanaKeypair.generate().publicKey;

//     txParams = new TransactionParameters({
//       outputUtxos: [compressUtxo1],
//       eventMerkleTreePubkey: mockPubkey,
//       transactionMerkleTreePubkey: mockPubkey,
//       senderSpl: mockPubkey,
//       senderSol: lightProvider.wallet.publicKey,
//       action: Action.COMPRESS,
//       hasher,
//       verifierIdl: IDL_LIGHT_PSP2IN2OUT,
//       account,
//     });

//     txParamsSol = new TransactionParameters({
//       outputUtxos: [compressUtxoSol],
//       eventMerkleTreePubkey: mockPubkey,
//       transactionMerkleTreePubkey: mockPubkey,
//       senderSpl: mockPubkey,
//       senderSol: lightProvider.wallet.publicKey,
//       action: Action.COMPRESS,
//       hasher,
//       verifierIdl: IDL_LIGHT_PSP2IN2OUT,
//       account,
//     });
//     lightProvider.solMerkleTree!.merkleTree = new MerkleTree(18, hasher, [
//       compressUtxo1.getCommitment(hasher),
//       // random invalid other commitment
//       hasher.poseidonHashString(["123124"]),
//     ]);

//     assert.equal(
//       lightProvider.solMerkleTree?.merkleTree.indexOf(
//         compressUtxo1.getCommitment(hasher),
//       ),
//       0,
//     );
//     rpc = new Rpc(mockPubkey3, mockPubkey, new BN(5000));
//     paramsDecompress = new TransactionParameters({
//       inputUtxos: [compressUtxo1],
//       eventMerkleTreePubkey: mockPubkey2,
//       transactionMerkleTreePubkey: mockPubkey2,
//       hasher,
//       recipientSpl: mockPubkey,
//       recipientSol: lightProvider.wallet.publicKey,
//       action: Action.DECOMPRESS,
//       rpc,
//       verifierIdl: IDL_LIGHT_PSP2IN2OUT,
//       account,
//     });
//     appData = { releaseSlot: BN_1 };
//     txParamsApp = new TransactionParameters({
//       inputUtxos: [
//         new Utxo({
//           index: 0,
//           hasher,
//           appData,
//           appDataIdl: IDL,
//           assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//           publicKey: account.pubkey,
//         }),
//       ],
//       eventMerkleTreePubkey: mockPubkey,
//       transactionMerkleTreePubkey: mockPubkey,
//       senderSpl: mockPubkey,
//       senderSol: lightProvider.wallet.publicKey,
//       action: Action.DECOMPRESS,
//       hasher,
//       rpc,
//       verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
//       account,
//     });
//     txParamsPoolType = new TransactionParameters({
//       inputUtxos: [
//         new Utxo({
//           index: 0,
//           hasher,
//           poolType: new BN("12312"),
//           assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//           publicKey: account.pubkey,
//         }),
//       ],
//       eventMerkleTreePubkey: mockPubkey,
//       transactionMerkleTreePubkey: mockPubkey,
//       senderSpl: mockPubkey,
//       senderSol: lightProvider.wallet.publicKey,
//       action: Action.DECOMPRESS,
//       hasher,
//       rpc,
//       verifierIdl: IDL_LIGHT_PSP2IN2OUT,
//       account,
//     });
//     txParamsPoolTypeOut = new TransactionParameters({
//       outputUtxos: [
//         new Utxo({
//           hasher,
//           poolType: new BN("12312"),
//           assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//           publicKey: account.pubkey,
//         }),
//       ],
//       eventMerkleTreePubkey: mockPubkey,
//       transactionMerkleTreePubkey: mockPubkey,
//       senderSpl: mockPubkey,
//       senderSol: lightProvider.wallet.publicKey,
//       action: Action.DECOMPRESS,
//       hasher,
//       rpc,
//       verifierIdl: IDL_LIGHT_PSP2IN2OUT,
//       account,
//     });
//     txParamsOutApp = new TransactionParameters({
//       outputUtxos: [
//         new Utxo({
//           hasher,
//           appData,
//           appDataIdl: IDL,
//           assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//           publicKey: account.pubkey,
//         }),
//       ],
//       eventMerkleTreePubkey: mockPubkey,
//       transactionMerkleTreePubkey: mockPubkey,
//       senderSpl: mockPubkey,
//       senderSol: lightProvider.wallet.publicKey,
//       action: Action.COMPRESS,
//       hasher,
//       // automatic encryption for app utxos is not implemented
//       encryptedUtxos: new Uint8Array(256).fill(1),
//       verifierIdl: IDL_LIGHT_PSP2IN2OUT,
//       account,
//     });
//   });

//   // should pass because no non-zero input utxo is provided
//   it("No in utxo test invalid root", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParams,
//     });
//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.root = new BN("123").toString();

//     await tx.getProof(account);
//   });

//   it("With in utxo test invalid root", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });
//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.root = new BN("123").toString();
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid tx integrity hash", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });
//     await tx.compile(lightProvider.hasher, account);

//     tx.proofInput.txIntegrityHash = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("No in utxo test invalid publicMintPubkey", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParams,
//     });
//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.publicMintPubkey = hashAndTruncateToCircuit(
//       SolanaKeypair.generate().publicKey.toBytes(),
//     );
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid publicMintPubkey", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });
//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.publicMintPubkey = hashAndTruncateToCircuit(
//       SolanaKeypair.generate().publicKey.toBytes(),
//     );
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   // should succeed because no public spl amount is provided thus mint is not checked
//   it("No public spl amount test invalid publicMintPubkey", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParamsSol,
//     });
//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.publicMintPubkey = hashAndTruncateToCircuit(
//       SolanaKeypair.generate().publicKey.toBytes(),
//     );
//     await tx.getProof(account);
//   });

//   it("With in utxo test invalid merkle proof path elements", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });
//     await tx.compile(lightProvider.hasher, account);

//     tx.proofInput.inPathElements[0] =
//       lightProvider.solMerkleTree?.merkleTree.path(1).pathElements;
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid merkle proof path index", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });
//     await tx.compile(lightProvider.hasher, account);

//     tx.proofInput.inPathIndices[0] = 1;
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid inPrivateKey", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     // tx.proofInput.inPrivateKey[0] = new BN("123").toString();
//     await chai.assert.isRejected(
//       tx.getProof(new Account({ hasher })),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid publicAmountSpl", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.publicAmountSpl = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid publicAmountSol", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.publicAmountSol = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid publicAmountSpl", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParamsSol,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.publicAmountSpl = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid publicUtxoHash", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     console.log();

//     tx.proofInput.publicUtxoHash[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid inAmount", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.inAmount[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid outAmount", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.outAmount[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid inBlinding", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.inBlinding[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid outBlinding", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.outBlinding[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid outPubkey", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.outPubkey[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid assetPubkeys", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     for (let i = 0; i < 3; i++) {
//       tx.proofInput.assetPubkeys[i] = hashAndTruncateToCircuit(
//         SolanaKeypair.generate().publicKey.toBytes(),
//       );

//       await chai.assert.isRejected(
//         tx.getProof(account),
//         TransactionErrorCode.PROOF_GENERATION_FAILED,
//       );
//     }
//   });

//   // this fails because the system verifier does not allow
//   it("With in utxo test invalid inAppDataHash", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParamsApp,
//       appParams: { mock: "1231", verifierIdl: IDL_LIGHT_PSP2IN2OUT },
//     });

//     await tx.compile(lightProvider.hasher, account);
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   // this works because the system verifier does not check output utxos other than commit hashes being well-formed and the sum
//   it("With out utxo test inAppDataHash", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParamsOutApp,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     await tx.getProof(account);
//   });

//   // this fails because it's inconsistent with the utxo
//   it("With in utxo test invalid outAppDataHash", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.outAppDataHash[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid pooltype", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParamsPoolType,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With out utxo test invalid pooltype", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParamsPoolTypeOut,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid inPoolType", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.inPoolType[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid outPoolType", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.outPoolType[0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid inIndices", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);

//     tx.proofInput.inIndices[0][0][0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid inIndices", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     chai.assert.notEqual(tx.proofInput.outIndices[1][1][1].toString(), "1");
//     tx.proofInput.inIndices[1][1][1] = "1";

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid outIndices", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);

//     tx.proofInput.outIndices[0][0][0] = new BN("123").toString();

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("With in utxo test invalid outIndices", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: paramsDecompress,
//     });

//     await tx.compile(lightProvider.hasher, account);
//     chai.assert.notEqual(tx.proofInput.outIndices[1][1][1].toString(), "1");
//     tx.proofInput.outIndices[1][1][1] = "1";

//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });
// });

// // TODO: check more specific errors in tests
// describe("App system circuit tests", () => {
//   let lightProvider: LightProvider;
//   before(async () => {
//     lightProvider = await LightProvider.loadMock();
//     hasher = await WasmFactory.getInstance();
//     account = new Account({ hasher, seed: seed32 });
//     await account.getEddsaPublicKey();
//     const compressAmount = 20_000;
//     const compressFeeAmount = 10_000;
//     compressUtxo1 = new Utxo({
//       hasher,
//       assets: [FEE_ASSET, MINT],
//       amounts: [new BN(compressFeeAmount), new BN(compressAmount)],
//       publicKey: account.pubkey,
//       assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//     });
//     mockPubkey = SolanaKeypair.generate().publicKey;
//     const rpcPubkey = SolanaKeypair.generate().publicKey;

//     lightProvider = await LightProvider.loadMock();
//     txParams = new TransactionParameters({
//       outputUtxos: [compressUtxo1],
//       eventMerkleTreePubkey: mockPubkey,
//       transactionMerkleTreePubkey: mockPubkey,
//       senderSpl: mockPubkey,
//       senderSol: lightProvider.wallet.publicKey,
//       action: Action.COMPRESS,
//       hasher,
//       verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
//       account,
//     });

//     rpc = new Rpc(rpcPubkey, mockPubkey, new BN(5000));
//     txParamsApp = new TransactionParameters({
//       inputUtxos: [
//         new Utxo({
//           hasher,
//           appData,
//           appDataIdl: IDL,
//           assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//           publicKey: account.pubkey,
//         }),
//       ],
//       eventMerkleTreePubkey: mockPubkey,
//       transactionMerkleTreePubkey: mockPubkey,
//       senderSpl: mockPubkey,
//       senderSol: lightProvider.wallet.publicKey,
//       action: Action.DECOMPRESS,
//       hasher,
//       rpc,
//       verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
//       account,
//     });
//   });

//   it("No in utxo test invalid transactionHash", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParams,
//       appParams: { mock: "123", verifierIdl: IDL_LIGHT_PSP2IN2OUT },
//     });
//     await tx.compile(lightProvider.hasher, account);

//     tx.proofInput.transactionHash = new BN("123").toString();
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });

//   it("No in utxo test invalid transactionHash", async () => {
//     const tx: Transaction = new Transaction({
//       ...(await lightProvider.getRootIndex()),
//       solMerkleTree: lightProvider.solMerkleTree!,
//       params: txParamsApp,
//       appParams: { mock: "123", verifierIdl: IDL_LIGHT_PSP2IN2OUT },
//     });
//     await tx.compile(lightProvider.hasher, account);
//     tx.proofInput.publicAppVerifier = new BN("123").toString();
//     await chai.assert.isRejected(
//       tx.getProof(account),
//       TransactionErrorCode.PROOF_GENERATION_FAILED,
//     );
//   });
// });
