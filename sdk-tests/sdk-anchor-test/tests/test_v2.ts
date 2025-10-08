import * as anchor from "@coral-xyz/anchor";
import { Program, web3 } from "@coral-xyz/anchor";
import idl from "../target/idl/sdk_anchor_test.json";
import {
  bn,
  CompressedAccountWithMerkleContext,
  createRpc,
  deriveAddressSeedV2,
  deriveAddressV2,
  PackedAccounts,
  Rpc,
  sleep,
  SystemAccountMetaConfig,
} from "@lightprotocol/stateless.js";
const path = require("path");
const os = require("os");
require("dotenv").config();

const anchorWalletPath = path.join(os.homedir(), ".config/solana/id.json");
process.env.ANCHOR_WALLET = anchorWalletPath;
process.env.ANCHOR_PROVIDER_URL = "http://localhost:8899";

describe("sdk-anchor-test-v2", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const programId = new web3.PublicKey(
    "2tzfijPBGbrR5PboyFUFKzfEoLTwdDSHUjANCw929wyt"
  );
  const program = anchor.workspace.sdkAnchorTest;
  const coder = new anchor.BorshCoder(idl as anchor.Idl);

  it("create, update, and close compressed account (v2)", async () => {
    let signer = new web3.Keypair();
    let rpc = createRpc(
      "http://127.0.0.1:8899",
      "http://127.0.0.1:8784",
      "http://127.0.0.1:3001",
      {
        commitment: "confirmed",
      }
    );

    // Get existing tree infos
    const existingTreeInfos = await rpc.getStateTreeInfos();
    console.log("Available tree infos:");
    existingTreeInfos.forEach((info) => {
      console.log(`  Tree: ${info.tree.toBase58()}, Type: ${info.treeType}`);
    });

    let lamports = web3.LAMPORTS_PER_SOL;
    await rpc.requestAirdrop(signer.publicKey, lamports);
    await sleep(2000);

    // Use an actual existing state tree from the environment
    const stateTreeInfo = existingTreeInfos.find(
      (info) => info.treeType === 2 || info.treeType === 3
    ); // StateV1 or StateV2
    if (!stateTreeInfo) {
      throw new Error("No state tree available");
    }
    const outputQueue = stateTreeInfo.queue;

    const addressTreeInfo = await rpc.getAddressTreeInfoV2();
    const addressTree = addressTreeInfo.tree;

    const name = "test-account";
    const accountSeed = new TextEncoder().encode("compressed");
    const nameSeed = new TextEncoder().encode(name);
    const seed = deriveAddressSeedV2([accountSeed, nameSeed]);
    const address = deriveAddressV2(seed, addressTree, program.programId);

    console.log("Creating compressed account with name:", name);
    await createCompressedAccount(
      rpc,
      addressTree,
      address,
      program,
      outputQueue,
      signer,
      name
    );
    await sleep(2000);

    let compressedAccount = await rpc.getCompressedAccount(
      bn(address.toBytes())
    );
    console.log("Created account:", compressedAccount);

    // Update the account with new nested data
    const newNestedData = {
      one: 10,
      two: 20,
      three: 30,
      four: 40,
      five: 50,
      six: 60,
      seven: 70,
      eight: 80,
      nine: 90,
      ten: 100,
      eleven: 110,
      twelve: 120,
    };

    console.log("Updating compressed account with new nested data");
    await updateCompressedAccount(
      rpc,
      compressedAccount,
      program,
      outputQueue,
      signer,
      coder,
      newNestedData
    );
    await sleep(2000);

    compressedAccount = await rpc.getCompressedAccount(bn(address.toBytes()));
    console.log("Updated account:", compressedAccount);

    // Close the account
    console.log("Closing compressed account");
    await closeCompressedAccount(
      rpc,
      compressedAccount,
      program,
      outputQueue,
      signer,
      coder
    );
    await sleep(2000);

    const closedAccount = await rpc.getCompressedAccount(bn(address.toBytes()));
    console.log("Closed account:", closedAccount);
  });
});

async function createCompressedAccount(
  rpc: Rpc,
  addressTree: web3.PublicKey,
  address: web3.PublicKey,
  program: Program<SdkAnchorTest>,
  outputMerkleTree: web3.PublicKey,
  signer: web3.Keypair,
  name: string
) {
  const proofRpcResult = await rpc.getValidityProofV0(
    [],
    [
      {
        tree: addressTree,
        queue: addressTree,
        address: bn(address.toBytes()),
      },
    ]
  );
  const systemAccountConfig = SystemAccountMetaConfig.new(program.programId);
  let remainingAccounts =
    PackedAccounts.newWithSystemAccountsV2(systemAccountConfig);

  const addressMerkleTreePubkeyIndex =
    remainingAccounts.insertOrGet(addressTree);
  const packedAddressTreeInfo = {
    addressMerkleTreePubkeyIndex,
    addressQueuePubkeyIndex: addressMerkleTreePubkeyIndex,
    rootIndex: proofRpcResult.rootIndices[0],
  };
  const outputMerkleTreeIndex = remainingAccounts.insertOrGet(outputMerkleTree);

  let proof = {
    0: proofRpcResult.compressedProof,
  };
  const computeBudgetIx = web3.ComputeBudgetProgram.setComputeUnitLimit({
    units: 1000000,
  });
  let tx = await program.methods
    .createCompressedAccountV2(
      proof,
      packedAddressTreeInfo,
      outputMerkleTreeIndex,
      name
    )
    .accounts({
      signer: signer.publicKey,
    })
    .preInstructions([computeBudgetIx])
    .remainingAccounts(remainingAccounts.toAccountMetas().remainingAccounts)
    .signers([signer])
    .transaction();
  tx.recentBlockhash = (await rpc.getRecentBlockhash()).blockhash;
  tx.sign(signer);
  console.log("tx ", tx.instructions[1].keys);
  const sig = await rpc.sendTransaction(tx, [signer]);
  await rpc.confirmTransaction(sig);
  console.log("Created compressed account", sig);
}

async function updateCompressedAccount(
  rpc: Rpc,
  compressedAccount: CompressedAccountWithMerkleContext,
  program: Program,
  outputMerkleTree: web3.PublicKey,
  signer: web3.Keypair,
  coder: anchor.BorshCoder,
  nestedData: any
) {
  const proofRpcResult = await rpc.getValidityProofV0(
    [
      {
        hash: compressedAccount.hash,
        tree: compressedAccount.treeInfo.tree,
        queue: compressedAccount.treeInfo.queue,
      },
    ],
    []
  );
  const systemAccountConfig = SystemAccountMetaConfig.new(program.programId);
  let remainingAccounts =
    PackedAccounts.newWithSystemAccountsV2(systemAccountConfig);

  const merkleTreePubkeyIndex = remainingAccounts.insertOrGet(
    compressedAccount.treeInfo.tree
  );
  const queuePubkeyIndex = remainingAccounts.insertOrGet(
    compressedAccount.treeInfo.queue
  );
  const outputMerkleTreeIndex = remainingAccounts.insertOrGet(outputMerkleTree);
  const compressedAccountMeta = {
    treeInfo: {
      rootIndex: proofRpcResult.rootIndices[0],
      proveByIndex: proofRpcResult.proveByIndices[0],
      merkleTreePubkeyIndex,
      queuePubkeyIndex,
      leafIndex: compressedAccount.leafIndex,
    },
    address: compressedAccount.address,
    outputStateTreeIndex: outputMerkleTreeIndex,
  };

  // Decode current account state
  const myCompressedAccount = coder.types.decode(
    "MyCompressedAccount",
    compressedAccount.data.data
  );

  let proof = {
    0: proofRpcResult.compressedProof,
  };
  const computeBudgetIx = web3.ComputeBudgetProgram.setComputeUnitLimit({
    units: 1000000,
  });
  let tx = await program.methods
    .updateCompressedAccountV2(
      proof,
      myCompressedAccount,
      compressedAccountMeta,
      nestedData
    )
    .accounts({
      signer: signer.publicKey,
    })
    .preInstructions([computeBudgetIx])
    .remainingAccounts(remainingAccounts.toAccountMetas().remainingAccounts)
    .signers([signer])
    .transaction();
  tx.recentBlockhash = (await rpc.getRecentBlockhash()).blockhash;
  tx.sign(signer);

  const sig = await rpc.sendTransaction(tx, [signer]);
  await rpc.confirmTransaction(sig);
  console.log("Updated compressed account", sig);
}

async function closeCompressedAccount(
  rpc: Rpc,
  compressedAccount: CompressedAccountWithMerkleContext,
  program: Program,
  outputMerkleTree: web3.PublicKey,
  signer: web3.Keypair,
  coder: anchor.BorshCoder
) {
  const proofRpcResult = await rpc.getValidityProofV0(
    [
      {
        hash: compressedAccount.hash,
        tree: compressedAccount.treeInfo.tree,
        queue: compressedAccount.treeInfo.queue,
      },
    ],
    []
  );
  const systemAccountConfig = SystemAccountMetaConfig.new(program.programId);
  let remainingAccounts =
    PackedAccounts.newWithSystemAccountsV2(systemAccountConfig);

  const merkleTreePubkeyIndex = remainingAccounts.insertOrGet(
    compressedAccount.treeInfo.tree
  );
  const queuePubkeyIndex = remainingAccounts.insertOrGet(
    compressedAccount.treeInfo.queue
  );
  const outputMerkleTreeIndex = remainingAccounts.insertOrGet(outputMerkleTree);

  const compressedAccountMeta = {
    treeInfo: {
      rootIndex: proofRpcResult.rootIndices[0],
      proveByIndex: proofRpcResult.proveByIndices[0],
      merkleTreePubkeyIndex,
      queuePubkeyIndex,
      leafIndex: compressedAccount.leafIndex,
    },
    address: compressedAccount.address,
    outputStateTreeIndex: outputMerkleTreeIndex,
  };

  // Decode current account state
  const myCompressedAccount = coder.types.decode(
    "MyCompressedAccount",
    compressedAccount.data.data
  );

  let proof = {
    0: proofRpcResult.compressedProof,
  };
  const computeBudgetIx = web3.ComputeBudgetProgram.setComputeUnitLimit({
    units: 1000000,
  });
  let tx = await program.methods
    .closeCompressedAccountV2(proof, myCompressedAccount, compressedAccountMeta)
    .accounts({
      signer: signer.publicKey,
    })
    .preInstructions([computeBudgetIx])
    .remainingAccounts(remainingAccounts.toAccountMetas().remainingAccounts)
    .signers([signer])
    .transaction();
  tx.recentBlockhash = (await rpc.getRecentBlockhash()).blockhash;
  tx.sign(signer);

  const sig = await rpc.sendTransaction(tx, [signer]);
  await rpc.confirmTransaction(sig);
  console.log("Closed compressed account", sig);
}
