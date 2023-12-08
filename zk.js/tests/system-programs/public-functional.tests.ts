import * as anchor from "@coral-xyz/anchor";
import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  Keypair as SolanaKeypair,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { Idl } from "@coral-xyz/anchor";
const token = require("@solana/spl-token");

// TODO: add and use namespaces in SDK
import {
  Utxo,
  LOOK_UP_TABLE,
  ADMIN_AUTH_KEYPAIR,
  AUTHORITY,
  MINT,
  Provider,
  AUTHORITY_ONE,
  createTestAccounts,
  userTokenAccount,
  FEE_ASSET,
  confirmConfig,
  User,
  TestRpc,
  TestTransaction,
  IDL_LIGHT_PSP2IN2OUT,
  IDL_LIGHT_PSP10IN2OUT,
  IDL_LIGHT_PSP2IN2OUT_STORAGE,
  Account,
  airdropSol,
  MerkleTreeConfig,
  RPC_FEE,
  BN_0,
  airdropSplToAssociatedTokenAccount,
  getSystemProof,
  createSystemProofInputs,
  createSolanaInstructions,
  getSolanaRemainingAccounts,
  CompressTransactionInput,
  createCompressTransaction,
  prepareAccounts,
  getVerifierProgramId,
  createDecompressTransaction,
  DecompressTransactionInput,
  syncInputUtxosMerkleProofs,
  createOutUtxo,
  OutUtxo,
  IDL_COMPRESSED_TOKEN_PROGRAM,
  merkleTreeProgramId,
  getTokenAuthorityPda,
  sleep,
  getSignerAuthorityPda,
  ProviderError,
  PublicTransactionIndexerEventBeet,
  PublicTransactionIndexerEventAnchor,
  PublicTestRpc,
  remainingAccount,
  createTransaction,
  TransactionInput,
  Action,
} from "../../src";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createInitializeMint2Instruction,
  getAssociatedTokenAddress,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { assert } from "chai";
import { SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";
import {BN} from  "@coral-xyz/anchor";
let WASM: LightWasm;
let RPC: PublicTestRpc;
let ACCOUNT: Account, ACCOUNT2: Account;

// TODO: remove deprecated function calls
describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  anchor.setProvider(provider);
  const compressedTokenProgram = new anchor.Program(IDL_COMPRESSED_TOKEN_PROGRAM,getVerifierProgramId(IDL_COMPRESSED_TOKEN_PROGRAM), provider )
  let authorityKeypair = Keypair.generate();
  let mintKeypair = Keypair.generate();
  const deriveAuthorityPda = (authority: PublicKey, mint: PublicKey): PublicKey => {
    let [pubkey,] = PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("authority"), authority.toBuffer(), mint.toBuffer()],
      getVerifierProgramId(IDL_COMPRESSED_TOKEN_PROGRAM),
    );
    return pubkey;
  };
  const authorityPda = deriveAuthorityPda(authorityKeypair.publicKey, mintKeypair.publicKey);

  before("init test setup Merkle tree lookup table etc", async () => {
    await createTestAccounts(provider.connection, userTokenAccount);

    WASM = await WasmFactory.getInstance();
    // let data = [215, 14, 158, 183, 149, 177, 250, 102, 242, 9, 248, 116, 252, 35, 233, 7, 209, 142, 128, 28, 205, 129, 198, 228, 231, 219, 112, 243, 178, 168, 241, 144];
    // let hash =  WASM.poseidonHash([new BN(data, 32, "le")]);
    // console.log("hash ", hash);
    // process.exit(0);
    const seed = bs58.encode(new Uint8Array(32).fill(1));
    const seed2 = bs58.encode(new Uint8Array(32).fill(2));
    ACCOUNT = Account.createFromSeed(WASM, seed);
    ACCOUNT2 = Account.createFromSeed(WASM, seed2);

    const rpcRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(rpcRecipientSol, 2e9);

    RPC = new PublicTestRpc({connection: provider.connection , lightWasm: WASM});
    await airdropSol({
      connection: provider.connection,
      lamports: 1000 * 1e9,
      recipientPublicKey: authorityKeypair.publicKey,
    });
  });
  
  it.only(" create mint", async () => {
   
    let createAccountInstruction = SystemProgram.createAccount({
      fromPubkey: authorityKeypair.publicKey,
      lamports: await provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE),
      newAccountPubkey: mintKeypair.publicKey,
      programId: TOKEN_PROGRAM_ID,
      space: MINT_SIZE,
    })
    


    let createMintInstruciton = createInitializeMint2Instruction(mintKeypair.publicKey, 2, authorityPda, null, TOKEN_PROGRAM_ID);
    const transferInstruction = anchor.web3.SystemProgram.transfer({
      fromPubkey: authorityKeypair.publicKey,
      toPubkey: authorityPda,
      lamports: await provider.connection.getMinimumBalanceForRentExemption(80) + await provider.connection.getMinimumBalanceForRentExemption(165),
    });
 
    let ix = await compressedTokenProgram.methods.createMint().accounts({
      feePayer: authorityKeypair.publicKey,
      authority: authorityKeypair.publicKey,
      mint: mintKeypair.publicKey,
      authorityPda,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      merkleTreeProgram: merkleTreeProgramId,
      tokenAuthority: getTokenAuthorityPda(),
      merkleTreePdaToken: MerkleTreeConfig.getSplPoolPdaToken(mintKeypair.publicKey),
      registeredAssetPoolPda: MerkleTreeConfig.getSplPoolPda(mintKeypair.publicKey),
      merkleTreeAuthorityPda: MerkleTreeConfig.getMerkleTreeAuthorityPda(),
      registeredPoolTypePda: MerkleTreeConfig.getPoolTypePda(new Uint8Array(32).fill(0)),
    })
    .signers([
      authorityKeypair,
      mintKeypair,
    ]).instruction();
    const transaction = new Transaction()
    .add(
      createAccountInstruction,
    ).add(
      createMintInstruciton
    ).add(transferInstruction)
    .add(ix);
    try {
    const res = await sendAndConfirmTransaction(provider.connection, transaction, [authorityKeypair, mintKeypair], confirmConfig);
    console.log(res)
    }
    catch (e) {
      console.log(e)
    }
  });

  it.only("Mint to", async () => {
    const registerVerifier = async () => {
      let merkleTreeConfig = new MerkleTreeConfig({payer: ADMIN_AUTH_KEYPAIR, anchorProvider: provider});
      let x = await merkleTreeConfig.registerVerifier(getVerifierProgramId(IDL_COMPRESSED_TOKEN_PROGRAM));
      console.log("registered verifier ", x)
    }
    await registerVerifier();
    const mint  = async () => {
      let tx = await compressedTokenProgram.methods.mintTo(
        [ACCOUNT.keypair.publicKey.toArray("be", 32), ACCOUNT.keypair.publicKey.toArray("be", 32)],
        [new anchor.BN(100), new anchor.BN(101)],
      ).accounts({
        feePayer: authorityKeypair.publicKey,
        authority: authorityKeypair.publicKey,
        mint: mintKeypair.publicKey,
        authorityPda,
        merkleTreePdaToken: MerkleTreeConfig.getSplPoolPdaToken(mintKeypair.publicKey),
        tokenProgram: TOKEN_PROGRAM_ID,
        merkleTreeProgram: merkleTreeProgramId,
        noopProgram: SPL_NOOP_PROGRAM_ID,
        merkleTreeSet: MerkleTreeConfig.getTransactionMerkleTreePda(),
        registeredVerifierPda: MerkleTreeConfig.getRegisteredVerifierPda(getVerifierProgramId(IDL_COMPRESSED_TOKEN_PROGRAM)),
        merkleTreeAuthority: getSignerAuthorityPda(merkleTreeProgramId, getVerifierProgramId(IDL_COMPRESSED_TOKEN_PROGRAM)),
      }).signers([
        authorityKeypair,
      ]).preInstructions([      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ]).transaction();
      let data
      try {
        let txHash = await sendAndConfirmTransaction(provider.connection, tx, [authorityKeypair], confirmConfig);
        console.log("txHash ", txHash)
      }
      catch (e) {
        console.log(e)
      }
    }
    const data = await mint()

    const utxos = await RPC.getAssetsByOwner(ACCOUNT.keypair.publicKey.toString());
    console.log("utxos ", utxos[0])
    assert.equal(utxos.length, 2);
    assert.equal(utxos[0].amounts[1].toNumber(), 100);
    assert.equal(utxos[1].amounts[1].toNumber(), 101);

    console.log("utxo 0 version", new anchor.BN(utxos[0].transactionVersion).toArray("be", 32))
    console.log("utxo 0 amount", new anchor.BN(utxos[0].amounts[0]).toArray("be", 32))
    console.log("utxo 0 amount", new anchor.BN(utxos[0].amounts[1]).toArray("be", 32))
    console.log("utxo 0 owner", new anchor.BN(utxos[0].publicKey).toArray("be", 32))
    console.log("utxo 0 blinding", new anchor.BN(utxos[0].blinding).toArray("be", 32))
    console.log("utxo 0 asset", new anchor.BN(utxos[0].assetsCircuit[0]).toArray("be", 32))
    console.log("utxo 0 asset", new anchor.BN(utxos[0].assetsCircuit[1]).toArray("be", 32))


//     '10360557237825235488097282037625996457621919477735467906377568872108966350594'
// '16180391079141022144172006926116243706152468827752959671432316270411413007669'

  });

  it.only("Compressed Token Transfer (2in2out)", async () => {
    await performCompressedTokenTransfer({senderAccount: ACCOUNT, recipientAccount: ACCOUNT2});
  });


  const performCompressedTokenTransfer = async ({senderAccount, recipientAccount}: {senderAccount: Account; recipientAccount: Account;}) => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }
    const verifierIdl = IDL_COMPRESSED_TOKEN_PROGRAM;

    // const lightProvider = await Provider.init({
    //   wallet: ADMIN_AUTH_KEYPAIR,
    //   rpc: RPC,
    //   confirmConfig,
    // });
    let senderUtxos = await RPC.getAssetsByOwner(senderAccount.keypair.publicKey.toString());
    const inputUtxos: Utxo[] = [senderUtxos[0]];
    
    const outputUtxo = createOutUtxo({
      lightWasm: WASM,
      assets: senderUtxos[0].assets,
      amounts: [
        BN_0,
        inputUtxos[0].amounts[1],
      ],
      publicKey: recipientAccount.keypair.publicKey,
      blinding: BN_0,
    });

    // const compressUtxo = spl
    //   ? createOutUtxo({
    //       lightWasm: WASM,
    //       assets: [FEE_ASSET, MINT],
    //       amounts: [
    //         new anchor.BN(compressFeeAmount),
    //         new anchor.BN(compressAmount),
    //       ],
    //       publicKey: ACCOUNT.keypair.publicKey,
    //     })
    //   : createOutUtxo({
    //       lightWasm: WASM,
    //       amounts: [new anchor.BN(compressFeeAmount)],
    //       publicKey: ACCOUNT.keypair.publicKey,
    //       assets: [FEE_ASSET],
    //     });

    const compressTransactionInput: TransactionInput = {
      lightWasm: WASM,
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      rpcPublicKey: ADMIN_AUTH_KEYPAIR.publicKey,
      systemPspId: getVerifierProgramId(verifierIdl),
      account: ACCOUNT,
      inputUtxos,
      outputUtxos: [outputUtxo],
      isPublic: true,
      rpcFee: BN_0
    };

    const compressTransaction = await createTransaction(
      compressTransactionInput,
    );

    const { root, index: rootIndex } = (await RPC.getMerkleRoot(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    ))!;

    const systemProofInputs = createSystemProofInputs({
      root,
      transaction: compressTransaction,
      lightWasm: WASM,
      account: ACCOUNT,
    });
    // console.log("systemProofInputs ", systemProofInputs)
    const systemProof = await getSystemProof({
      account: ACCOUNT,
      inputUtxos: compressTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });
    console.log("systemProof ", systemProof.parsedPublicInputsObject)
    const remainingSolanaAccounts: remainingAccount[] = new Array(4).fill({isSigner: false, isWritable: true, pubkey: MerkleTreeConfig.getTransactionMerkleTreePda()})//,MerkleTreeConfig.getTransactionMerkleTreePda()]

    const accounts = prepareAccounts({
      transactionAccounts: compressTransaction.public.accounts,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
    });
    // console.log("authority accounts generated ", accounts.authority);
    // console.log("authority accounts derived ", getSignerAuthorityPda(merkleTreeProgramId, getVerifierProgramId(IDL_COMPRESSED_TOKEN_PROGRAM)));
    // process.exit(0);
    let serializedOutUtxo = (await new anchor.BorshCoder(IDL_COMPRESSED_TOKEN_PROGRAM).accounts.encode("transferOutputUtxo", {
      owner: new anchor.BN(outputUtxo.publicKey),
      amounts: outputUtxo.amounts,
      splAssetMint: outputUtxo.assets[1],
      metaHash: null,
      address: null,
    })).subarray(8);
    // console.log("serializedOutUtxo ", serializedOutUtxo);
    // let decoded = await new anchor.BorshCoder(IDL_COMPRESSED_TOKEN_PROGRAM).accounts.decodeUnchecked("transferOutputUtxo", Buffer.from([...new Array(8), ...serializedOutUtxo]));
    // TODO: add more Merkle tree remaining accounts and find an automated way to do so
    // createSolanaInstructionsWithAccounts
    const instructions = await createSolanaInstructions({
      action: Action.TRANSFER,
      rootIndex,
      systemProof,
      remainingSolanaAccounts: remainingSolanaAccounts as any,
      accounts,
      publicTransactionVariables: compressTransaction.public,
      systemPspIdl: verifierIdl,
      instructionName: "transfer2In2Out",
      customInputs: {
        outUtxo: [
          serializedOutUtxo,
          null,
        ]
      }
    });
    console.log("instructions ", instructions[0].keys)
    try {
      const txHash = await sendAndConfirmTransaction(provider.connection, new Transaction().add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 })).add(instructions[0]), [ADMIN_AUTH_KEYPAIR], confirmConfig);
      console.log("txHash ", txHash)
    }
    catch (e) {
      console.log(e)
      throw e;
    }
    let recpientBalance = await RPC.getAssetsByOwner(recipientAccount.keypair.publicKey.toString());
    console.log("recpientBalance ", recpientBalance)
    console.log("RPC utxos", RPC.utxos)
    assert.deepEqual(recpientBalance[0].amounts[1].toNumber(), 100);
    // check that I rebuilt the correct tree
    (await RPC.getMerkleRoot(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    ))!;
    // check that utxo was inserted
    assert.equal(2, RPC.merkleTrees[0].merkleTree.indexOf(recpientBalance[0].utxoHash));
    // const transactionTester = new TestTransaction({
    //   transaction: compressTransaction,
    //   accounts,
    //   provider: lightProvider,
    // });
    // await transactionTester.getTestValues();
    // await lightProvider.sendAndConfirmSolanaInstructions(instructions);

    // // TODO: check why encryptedUtxo check doesn't work
    // await transactionTester.checkBalances(
    //   { publicInputs: systemProof.parsedPublicInputsObject },
    //   remainingSolanaAccounts,
    //   systemProofInputs,
    // );
  };

  const performDecompress = async ({
    outputUtxos,
    tokenProgram,
    message,
    spl,
    shuffleEnabled = true,
    verifierIdl,
  }: {
    outputUtxos: Array<OutUtxo>;
    tokenProgram: anchor.web3.PublicKey;
    message?: Buffer;
    spl?: boolean;
    shuffleEnabled: boolean;
    verifierIdl: Idl;
  }) => {
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      rpc: RPC,
      confirmConfig,
    });
    const user = await User.init({
      provider: lightProvider,
      account: ACCOUNT,
    });

    const origin = Keypair.generate();
    await airdropSol({
      connection: lightProvider.provider.connection,
      lamports: 1000 * 1e9,
      recipientPublicKey: origin.publicKey,
    });
    const ata = await getAssociatedTokenAddress(MINT, origin.publicKey);

    const decompressUtxo = user.balance.tokenBalances
      .get(tokenProgram.toBase58())!
      .utxos.values()
      .next().value;
    const {
      syncedUtxos,
      root,
      index: rootIndex,
    } = await syncInputUtxosMerkleProofs({
      inputUtxos: [decompressUtxo],
      merkleTreePublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      rpc: RPC,
    });
    // Running into memory issues with verifier one (10in2out) decompressing spl
    const decompressTransactionInput: DecompressTransactionInput = {
      lightWasm: WASM,
      mint: spl ? MINT : undefined,
      message,
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      recipientSpl: spl ? ata : undefined,
      recipientSol: origin.publicKey,
      rpcPublicKey: lightProvider.rpc.accounts.rpcPubkey,
      systemPspId: getVerifierProgramId(verifierIdl),
      account: ACCOUNT,
      inputUtxos: syncedUtxos,
      outputUtxos,
      rpcFee: user.provider.rpc.getRpcFee(true),
      ataCreationFee: spl ? spl : false,
    };

    const decompressTransaction = await createDecompressTransaction(
      decompressTransactionInput,
    );

    const systemProofInputs = createSystemProofInputs({
      transaction: decompressTransaction,
      lightWasm: WASM,
      account: ACCOUNT,
      root,
    });
    const systemProof = await getSystemProof({
      account: ACCOUNT,
      inputUtxos: decompressTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });

    const remainingSolanaAccounts = getSolanaRemainingAccounts(
      systemProof.parsedPublicInputsObject as any,
    );
    const accounts = prepareAccounts({
      transactionAccounts: decompressTransaction.public.accounts,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      rpcRecipientSol: lightProvider.rpc.accounts.rpcRecipientSol,
      signer: lightProvider.rpc.accounts.rpcPubkey,
    });
    // createSolanaInstructionsWithAccounts
    const instructions = await createSolanaInstructions({
      action: decompressTransaction.action,
      rootIndex,
      systemProof,
      remainingSolanaAccounts,
      accounts,
      publicTransactionVariables: decompressTransaction.public,
      systemPspIdl: verifierIdl,
    });
    const transactionTester = new TestTransaction({
      transaction: decompressTransaction,
      accounts,
      provider: lightProvider,
    });
    await transactionTester.getTestValues();
    await lightProvider.rpc.sendAndConfirmSolanaInstructions(
      instructions,
      lightProvider.provider.connection,
      undefined,
      undefined,
      lightProvider,
    );

    await transactionTester.checkBalances(
      { publicInputs: systemProof.parsedPublicInputsObject },
      remainingSolanaAccounts,
      systemProofInputs,
    );
  };
});
