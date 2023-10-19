// import {
//   Action,
//   BN_0,
//   Balance,
//   FEE_ASSET,
//   MINT,
//   ProgramUtxoBalance,
//   TokenUtxoBalance,
//   UserIndexedTransaction,
//   Utxo,
// } from "@lightprotocol/zk.js";
// // import { buildPoseidonOpt } from "@";
// import { PublicKey } from "@solana/web3.js";
// import { BN } from "@coral-xyz/anchor";
// const mockTx = {
//   blockTime: 123456789,
//   signer: PublicKey.default,
//   signature: "someSignature1",
//   to: PublicKey.default,
//   from: PublicKey.default,
//   toSpl: PublicKey.default,
//   fromSpl: PublicKey.default,
//   verifier: PublicKey.default,
//   relayerRecipientSol: PublicKey.default,
//   type: Action.TRANSFER,
//   changeSolAmount: BN_0,
//   publicAmountSol: BN_0,
//   publicAmountSpl: BN_0,
//   encryptedUtxos: Buffer.from("someData1"),
//   leaves: [
//     [1, 2, 3],
//     [4, 5, 6],
//   ],
//   firstLeafIndex: BN_0,
//   nullifiers: [BN_0],
//   relayerFee: BN_0,
//   message: Buffer.from("someMessage1"),
//   inSpentUtxos: [], // Array of Utxo objects
//   outSpentUtxos: [], // Array of Utxo objects
// };

// const mockUser = {
//   getTransactionHistory: async (): Promise<UserIndexedTransaction[]> => {
//     return Promise.resolve([mockTx] as UserIndexedTransaction[]); // Mock transaction history
//   },
//   getBalance: async (): Promise<Balance> => {
//     return Promise.resolve(1000 as Balance); // Mock balance
//   },
//   getAllUtxos: (): Utxo[] => [mockUtxo] as Utxo[], // Mock UTXOs
//   // Add other methods/properties as needed
// };
// let poseidon;
// (async () => await buildPoseidonOpt())();

// const deposit_utxo1 = new Utxo({
//   index: 0,
//   poseidon: poseidon,
//   assets: [FEE_ASSET, MINT],
//   amounts: [new BN(0), new BN(10)],
//   publicKey: account.pubkey,
//   assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//   verifierProgramLookupTable:
//     lightProvider.lookUpTables.verifierProgramLookupTable,
// });
// let deposit_utxoSol = new Utxo({
//   index: 0,
//   poseidon: poseidon,
//   assets: [FEE_ASSET, MINT],
//   amounts: [new BN(depositFeeAmount), BN_0],
//   publicKey: account.pubkey,
//   assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
//   verifierProgramLookupTable:
//     lightProvider.lookUpTables.verifierProgramLookupTable,
// });

// const mockUtxo = {
//   amounts: [BN_0, BN_0],
//   assets: [PublicKey.default, PublicKey.default],
//   assetsCircuit: [BN_0, BN_0],
//   blinding: BN_0,
//   publicKey: BN_0,
//   index: 0,
//   appData: {},
//   verifierAddress: PublicKey.default,
//   verifierAddressCircuit: BN_0,
//   appDataHash: BN_0,
//   poolType: BN_0,
//   includeAppData: false,
//   transactionVersion: "1.0.0",
//   splAssetIndex: BN_0,
//   verifierProgramIndex: BN_0,
//   isFillingUtxo: false,
// };

// const mockTokenUtxoBalance: TokenUtxoBalance = {
//   tokenData: {
//     symbol: "mockSymbol",
//     decimals: BN_0,
//     isNft: false,
//     isNative: false,
//     mint: PublicKey.default,
//   },
//   totalBalanceSpl: BN_0,
//   totalBalanceSol: BN_0,
//   utxos: new Map<string, Utxo>([
//     ["utxo1", mockUtxo], // Use the previously created mockUtxo
//   ]),
//   committedUtxos: new Map<string, Utxo>(),
//   spentUtxos: new Map<string, Utxo>(),
// };
