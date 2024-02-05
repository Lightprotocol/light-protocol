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

import {
  AnchorProvider,
  BN,
  BorshCoder,
  Program,
  setProvider,
  utils,
  web3,
} from "@coral-xyz/anchor";

// TODO: add and use namespaces in SDK
import {
  Utxo,
  LOOK_UP_TABLE,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  userTokenAccount,
  confirmConfig,
  Account,
  airdropSol,
  MerkleTreeConfig,
  BN_0,
  getSystemProof,
  createSystemProofInputs,
  createSolanaInstructions,
  prepareAccounts,
  getVerifierProgramId,
  createOutUtxo,
  IDL_PSP_COMPRESSED_TOKEN,
  merkleTreeProgramId,
  getTokenAuthorityPda,
  getSignerAuthorityPda,
  PublicTestRpc,
  remainingAccount,
  createTransaction,
  TransactionInput,
  Action,
  IDL_PSP_ACCOUNT_COMPRESSION,
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
let WASM: LightWasm;
let RPC: PublicTestRpc;
let ACCOUNT: Account, ACCOUNT2: Account;
const initializeIndexedArray = async ({
  feePayer,
  indexedArrayKeypair,
  connection,
}: {
  connection: Connection;
  feePayer: Keypair;
  indexedArrayKeypair: Keypair;
}) => {
  const space = 112120;
  const accountCompressionProgramId = getVerifierProgramId(
    IDL_PSP_ACCOUNT_COMPRESSION,
  );
  const accountCompressionProgram = new Program(
    IDL_PSP_ACCOUNT_COMPRESSION,
    accountCompressionProgramId,
  );
  const ix1 = SystemProgram.createAccount({
    fromPubkey: feePayer.publicKey,
    newAccountPubkey: indexedArrayKeypair.publicKey,
    space,
    lamports: await connection.getMinimumBalanceForRentExemption(space),
    programId: accountCompressionProgramId,
  });

  const ix2 = await accountCompressionProgram.methods
    .initializeIndexedArray(new BN(0), merkleTreeProgramId, null)
    .accounts({
      authority: feePayer.publicKey,
      indexedArray: indexedArrayKeypair.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
  const tx = new Transaction().add(ix1, ix2);
  try {
    const txHash = await sendAndConfirmTransaction(
      connection,
      tx,
      [feePayer, indexedArrayKeypair],
      confirmConfig,
    );
    console.log(
      "------------------ initialized indexed array ------------------",
    );
    console.log("txHash ", txHash);
  } catch (e) {
    console.log(e);
  }
};
const initializeMerkleTree = async ({
  feePayer,
  merkleTreeKeypair,
  connection,
}: {
  connection: Connection;
  feePayer: Keypair;
  merkleTreeKeypair: Keypair;
}) => {
  const space = 90480;
  const accountCompressionProgramId = getVerifierProgramId(
    IDL_PSP_ACCOUNT_COMPRESSION,
  );
  const accountCompressionProgram = new Program(
    IDL_PSP_ACCOUNT_COMPRESSION,
    accountCompressionProgramId,
  );
  const ix1 = SystemProgram.createAccount({
    fromPubkey: feePayer.publicKey,
    newAccountPubkey: merkleTreeKeypair.publicKey,
    space,
    lamports: await connection.getMinimumBalanceForRentExemption(space),
    programId: accountCompressionProgramId,
  });

  const ix2 = await accountCompressionProgram.methods
    .initializeConcurrentMerkleTree(new BN(0), merkleTreeProgramId, null)
    .accounts({
      authority: feePayer.publicKey,
      merkleTree: merkleTreeKeypair.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .instruction();
  const tx = new Transaction().add(ix1, ix2);
  try {
    const txHash = await sendAndConfirmTransaction(
      connection,
      tx,
      [feePayer, merkleTreeKeypair],
      confirmConfig,
    );
    console.log(
      "------------------ initialized merkle tree ------------------",
    );
    console.log("txHash ", txHash);
  } catch (e) {
    console.log(e);
  }
};
// TODO: remove deprecated function calls
describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const provider = AnchorProvider.local("http://127.0.0.1:8899", confirmConfig);
  setProvider(provider);
  const compressedTokenProgram = new Program(
    IDL_PSP_COMPRESSED_TOKEN,
    getVerifierProgramId(IDL_PSP_COMPRESSED_TOKEN),
    provider,
  );
  const authorityKeypair = Keypair.generate();
  const mintKeypair = Keypair.generate();
  const deriveAuthorityPda = (
    authority: PublicKey,
    mint: PublicKey,
  ): PublicKey => {
    const [pubkey] = PublicKey.findProgramAddressSync(
      [
        utils.bytes.utf8.encode("authority"),
        authority.toBuffer(),
        mint.toBuffer(),
      ],
      getVerifierProgramId(IDL_PSP_COMPRESSED_TOKEN),
    );
    return pubkey;
  };
  const authorityPda = deriveAuthorityPda(
    authorityKeypair.publicKey,
    mintKeypair.publicKey,
  );
  const merkleTreeKeyPair = Keypair.generate();
  const indexedArrayKeypair = Keypair.generate();
  before("init test setup Merkle tree lookup table etc", async () => {
    await createTestAccounts(provider.connection, userTokenAccount);

    WASM = await WasmFactory.getInstance();
    const seed = bs58.encode(new Uint8Array(32).fill(1));
    const seed2 = bs58.encode(new Uint8Array(32).fill(2));

    ACCOUNT = Account.createFromSeed(WASM, seed);
    ACCOUNT2 = Account.createFromSeed(WASM, seed2);

    const rpcRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(rpcRecipientSol, 2e9);

    RPC = new PublicTestRpc({
      connection: provider.connection,
      lightWasm: WASM,
      merkleTreePublicKey: merkleTreeKeyPair.publicKey,
    });
    await airdropSol({
      connection: provider.connection,
      lamports: 1000 * 1e9,
      recipientPublicKey: authorityKeypair.publicKey,
    });
    await initializeMerkleTree({
      feePayer: ADMIN_AUTH_KEYPAIR,
      merkleTreeKeypair: merkleTreeKeyPair,
      connection: provider.connection,
    });

    await initializeIndexedArray({
      feePayer: ADMIN_AUTH_KEYPAIR,
      indexedArrayKeypair,
      connection: provider.connection,
    });
  });

  it(" create mint", async () => {
    const createAccountInstruction = SystemProgram.createAccount({
      fromPubkey: authorityKeypair.publicKey,
      lamports:
        await provider.connection.getMinimumBalanceForRentExemption(MINT_SIZE),
      newAccountPubkey: mintKeypair.publicKey,
      programId: TOKEN_PROGRAM_ID,
      space: MINT_SIZE,
    });

    const createMintInstruciton = createInitializeMint2Instruction(
      mintKeypair.publicKey,
      2,
      authorityPda,
      null,
      TOKEN_PROGRAM_ID,
    );
    const transferInstruction = web3.SystemProgram.transfer({
      fromPubkey: authorityKeypair.publicKey,
      toPubkey: authorityPda,
      lamports:
        (await provider.connection.getMinimumBalanceForRentExemption(80)) +
        (await provider.connection.getMinimumBalanceForRentExemption(165)),
    });

    const ix = await compressedTokenProgram.methods
      .createMint()
      .accounts({
        feePayer: authorityKeypair.publicKey,
        authority: authorityKeypair.publicKey,
        mint: mintKeypair.publicKey,
        authorityPda,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        merkleTreeProgram: merkleTreeProgramId,
        tokenAuthority: getTokenAuthorityPda(),
        merkleTreePdaToken: MerkleTreeConfig.getSplPoolPdaToken(
          mintKeypair.publicKey,
        ),
        registeredAssetPoolPda: MerkleTreeConfig.getSplPoolPda(
          mintKeypair.publicKey,
        ),
        merkleTreeAuthorityPda: MerkleTreeConfig.getMerkleTreeAuthorityPda(),
        registeredPoolTypePda: MerkleTreeConfig.getPoolTypePda(
          new Uint8Array(32).fill(0),
        ),
      })
      .signers([authorityKeypair, mintKeypair])
      .instruction();
    const transaction = new Transaction()
      .add(createAccountInstruction)
      .add(createMintInstruciton)
      .add(transferInstruction)
      .add(ix);
    try {
      const res = await sendAndConfirmTransaction(
        provider.connection,
        transaction,
        [authorityKeypair, mintKeypair],
        confirmConfig,
      );
      console.log(res);
    } catch (e) {
      console.log(e);
    }
  });

  it("Mint to", async () => {
    const registerVerifier = async () => {
      const merkleTreeConfig = new MerkleTreeConfig({
        payer: ADMIN_AUTH_KEYPAIR,
        anchorProvider: provider,
      });
      const x = await merkleTreeConfig.registerVerifier(
        getVerifierProgramId(IDL_PSP_COMPRESSED_TOKEN),
      );
      console.log("registered verifier ", x);
    };

    await registerVerifier();

    const mint = async () => {
      const tx = await compressedTokenProgram.methods
        .mintTo(
          [
            ACCOUNT.keypair.publicKey.toArray("be", 32),
            ACCOUNT.keypair.publicKey.toArray("be", 32),
          ],
          [new BN(100), new BN(101)],
        )
        .accounts({
          feePayer: authorityKeypair.publicKey,
          authority: authorityKeypair.publicKey,
          mint: mintKeypair.publicKey,
          authorityPda,
          merkleTreePdaToken: MerkleTreeConfig.getSplPoolPdaToken(
            mintKeypair.publicKey,
          ),
          tokenProgram: TOKEN_PROGRAM_ID,
          merkleTreeProgram: merkleTreeProgramId,
          noopProgram: SPL_NOOP_PROGRAM_ID,
          merkleTreeSet: merkleTreeKeyPair.publicKey,
          registeredVerifierPda: MerkleTreeConfig.getRegisteredVerifierPda(
            getVerifierProgramId(IDL_PSP_COMPRESSED_TOKEN),
          ),
          merkleTreeAuthority: getSignerAuthorityPda(
            merkleTreeProgramId,
            getVerifierProgramId(IDL_PSP_COMPRESSED_TOKEN),
          ),
          accountCompression: getVerifierProgramId(IDL_PSP_ACCOUNT_COMPRESSION),
          pspAccountCompressionAuthority: getSignerAuthorityPda(
            getVerifierProgramId(IDL_PSP_ACCOUNT_COMPRESSION),
            getVerifierProgramId(IDL_PSP_COMPRESSED_TOKEN),
          ),
        })
        .signers([authorityKeypair])
        .preInstructions([
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ])
        .transaction();
      let data;
      try {
        const txHash = await sendAndConfirmTransaction(
          provider.connection,
          tx,
          [authorityKeypair],
          confirmConfig,
        );
        console.log("txHash ", txHash);
      } catch (e) {
        console.log(e);
      }
    };
    const data = await mint();

    const utxos = await RPC.getAssetsByOwner(
      ACCOUNT.keypair.publicKey.toString(),
    );
    console.log("utxos ", utxos[0]);
    assert.equal(utxos.length, 2);
    assert.equal(utxos[0].amounts[1].toNumber(), 100);
    assert.equal(utxos[1].amounts[1].toNumber(), 101);
    console.log("utxo 0 version", new BN(utxos[0].version).toArray("be", 32));
    console.log("utxo 0 amount", new BN(utxos[0].amounts[0]).toArray("be", 32));
    console.log("utxo 0 amount", new BN(utxos[0].amounts[1]).toArray("be", 32));
    console.log("utxo 0 owner", new BN(utxos[0].owner).toArray("be", 32));
    console.log("utxo 0 blinding", new BN(utxos[0].blinding).toArray("be", 32));
    await RPC.getMerkleRoot(merkleTreeKeyPair.publicKey);
  });

  it("Compressed Token Transfer (2in2out)", async () => {
    await performCompressedTokenTransfer({
      senderAccount: ACCOUNT,
      recipientAccount: ACCOUNT2,
    });
  });

  const performCompressedTokenTransfer = async ({
    senderAccount,
    recipientAccount,
  }: {
    senderAccount: Account;
    recipientAccount: Account;
  }) => {
    if (LOOK_UP_TABLE === undefined) {
      throw "undefined LOOK_UP_TABLE";
    }
    const verifierIdl = IDL_PSP_COMPRESSED_TOKEN;

    const senderUtxos = await RPC.getAssetsByOwner(
      senderAccount.keypair.publicKey.toString(),
    );
    const inputUtxos: Utxo[] = [senderUtxos[0]];

    const outputUtxo = createOutUtxo({
      lightWasm: WASM,
      assets: senderUtxos[0].assets,
      amounts: [BN_0, inputUtxos[0].amounts[1]],
      owner: recipientAccount.keypair.publicKey,
      blinding: BN_0,
      isPublic: true,
    });

    const transactionInput: TransactionInput = {
      lightWasm: WASM,
      merkleTreeSetPubkey: merkleTreeKeyPair.publicKey,
      rpcPublicKey: ADMIN_AUTH_KEYPAIR.publicKey,
      systemPspId: getVerifierProgramId(verifierIdl),
      account: ACCOUNT,
      inputUtxos,
      outputUtxos: [outputUtxo],
      isPublic: true,
      rpcFee: BN_0,
    };

    const transaction = await createTransaction(transactionInput);

    const { root, index: rootIndex } = (await RPC.getMerkleRoot(
      merkleTreeKeyPair.publicKey,
    ))!;

    const systemProofInputs = createSystemProofInputs({
      root,
      transaction: transaction,
      lightWasm: WASM,
      account: ACCOUNT,
    });
    // console.log("systemProofInputs ", systemProofInputs)
    const systemProof = await getSystemProof({
      account: ACCOUNT,
      inputUtxos: transaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });
    console.log("systemProof ", systemProof.parsedPublicInputsObject);
    // Remaining accounts layout:
    // all remainging accounts need to be set regardless whether less utxos are actually used
    // 0..NR_IN_Utxos: in utxos
    // NR_IN_Utxos..NR_IN_Utxos+NR_IN_Utxos: indexed arrays to nullify in utxos
    // NR_IN_Utxos+NR_IN_Utxos..NR_IN_Utxos+NR_IN_Utxos+NR_OUT_Utxos: out utxos
    const remainingSolanaAccounts: remainingAccount[] = [
      ...new Array(2).fill({
        isSigner: false,
        isWritable: true,
        pubkey: merkleTreeKeyPair.publicKey,
      }),
      ...new Array(2).fill({
        isSigner: false,
        isWritable: true,
        pubkey: indexedArrayKeypair.publicKey,
      }),
      ...new Array(2).fill({
        isSigner: false,
        isWritable: true,
        pubkey: merkleTreeKeyPair.publicKey,
      }),
    ];

    const accounts = prepareAccounts({
      transactionAccounts: transaction.public.accounts,
      merkleTreeSet: merkleTreeKeyPair.publicKey,
    });
    // pspAccountCompression -> accountCompressionProgram
    accounts["pspAccountCompression"] = getVerifierProgramId(
      IDL_PSP_ACCOUNT_COMPRESSION,
    );
    accounts["accountCompressionAuthority"] = getSignerAuthorityPda(
      getVerifierProgramId(IDL_PSP_ACCOUNT_COMPRESSION),
      getVerifierProgramId(IDL_PSP_COMPRESSED_TOKEN),
    );

    const serializedOutUtxo = (
      await new BorshCoder(IDL_PSP_COMPRESSED_TOKEN).accounts.encode(
        "transferOutputUtxo",
        {
          owner: new BN(outputUtxo.owner),
          amounts: outputUtxo.amounts,
          splAssetMint: outputUtxo.assets[1],
          metaHash: null,
          address: null,
        },
      )
    ).subarray(8);

    const instructions = await createSolanaInstructions({
      action: Action.TRANSFER,
      rootIndex,
      systemProof,
      remainingSolanaAccounts: remainingSolanaAccounts as any,
      accounts,
      publicTransactionVariables: transaction.public,
      systemPspIdl: verifierIdl,
      instructionName: "transfer2In2Out",
      customInputs: {
        outUtxo: [serializedOutUtxo, null],
        lowElementIndexes: [0],
      },
      removeZeroUtxos: true,
    });
    try {
      const txHash = await sendAndConfirmTransaction(
        provider.connection,
        new Transaction()
          .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }))
          .add(instructions[0]),
        [ADMIN_AUTH_KEYPAIR],
        confirmConfig,
      );
      console.log("txHash ", txHash);
    } catch (e) {
      console.log(e);
      throw e;
    }
    const recpientBalance = await RPC.getAssetsByOwner(
      recipientAccount.keypair.publicKey.toString(),
    );
    console.log("recpientBalance ", recpientBalance);
    console.log(
      "RPC utxos",
      RPC.utxos.map((utxo) => new BN(utxo.hash).toArray("be", 32)),
    );
    assert.deepEqual(recpientBalance[0].amounts[1].toNumber(), 100);
    // assert.deepEqual(recpientBalance[0].hash, outputUtxo.hash);

    // assert.deepEqual(recpientBalance[1].amounts[1].toNumber(), 101);
    // check that I rebuilt the correct tree
    (await RPC.getMerkleRoot(merkleTreeKeyPair.publicKey))!;
    // check that utxo was inserted
    assert.equal(
      2,
      RPC.merkleTrees[0].merkleTree.indexOf(recpientBalance[0].hash.toString()),
    );
  };
});
