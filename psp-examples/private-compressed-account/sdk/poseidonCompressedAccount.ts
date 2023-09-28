import {
  SystemProgram,
  PublicKey,
  VersionedTransaction,
  TransactionMessage,
  TransactionInstruction,
  Connection,
} from "@solana/web3.js";

import { MerkleTreeWithHistory, zeroValues } from "./merkleTreeWithHistory";
import {
  BN,
  web3,
  Idl,
  Program,
  AnchorProvider,
  utils,
} from "@coral-xyz/anchor";
import {
  confirmConfig,
  Action,
  User,
  ProgramParameters,
  confirmTransaction,
  Wallet,
  ConfirmOptions,
} from "@lightprotocol/zk.js";
import { Prover } from "@lightprotocol/prover.js";
import {MerkleTree} from "@lightprotocol/circuit-lib.js";
const path = require("path");

const getSubTreeHash = (subTrees: BN[], poseidon: any) => {
  const subHash0 = poseidon.F.toString(poseidon(subTrees.slice(0, 9)));
  const subHash1 = poseidon.F.toString(poseidon(subTrees.slice(9, 18)));
  return poseidon.F.toString(poseidon([subHash0, subHash1]));
};

export class PoseidonCompressedAccount {
  poseidon: any;
  merkleTree: MerkleTreeWithHistory;
  idl: Idl;
  pda_index: number;
  programId: web3.PublicKey;
  pdaPublicKey: web3.PublicKey;
  program?: Program;
  merkleTreeAccountInfo: any;
  prover?: Prover;
  user: User;

  constructor(
    poseidon: any,
    idl: Idl,
    pda_index: number,
    user?: User,
    merkleTree?: MerkleTreeWithHistory
  ) {
    this.poseidon = poseidon;
    this.idl = idl;
    pda_index = pda_index;
    this.programId = PoseidonCompressedAccount.findProgramId(idl);
    this.merkleTree = merkleTree
      ? merkleTree
      : new MerkleTreeWithHistory(18, poseidon);
    this.pdaPublicKey = PoseidonCompressedAccount.getMerkleTreeAccountPublicKey(
      idl,
      pda_index
    )[0];
    if (user) {
      this.initProgram(user.provider.provider);
      this.user = user;
    }
  }

  async initMerkleTreeAccount() {
    if (!this.program) throw new Error("Program not initialized");
    const ix = await this.program.methods
      .initCompressedAccountMerkleTree(this.pda_index)
      .accounts({
        compressedAccountMerkleTree: this.pdaPublicKey,
        systemProgram: SystemProgram.programId,
        signer: this.user.provider.wallet.publicKey,
      })
      .instruction();
    return sendAndConfirmInstructionWithWallet(
      [ix],
      this.user.provider.connection,
      this.user.provider.wallet
    );
  }

  initProgram(anchorProvider: AnchorProvider) {
    this.program = new Program(this.idl, this.programId, anchorProvider);
    return this.program;
  }

  static findProgramId(idl: Idl) {
    return new web3.PublicKey(
      idl.constants.find((c) => c.name === "PROGRAM_ID").value.slice(1, -1)
    );
  }

  static getMerkleTreeAccountPublicKey(idl: Idl, pda_index: number) {
    const pdaIndexBytes = new BN(pda_index).toBuffer("le", 8);
    const seed = utils.bytes.utf8.encode("compression_merkle_tree"); //Buffer.from(idl.constants.find((c) => c.name === "COMPRESSION_MERKLE_TREE_SEED").value);
    const programId = PoseidonCompressedAccount.findProgramId(idl);
    return PublicKey.findProgramAddressSync([pdaIndexBytes, seed], programId);
  }

  async getMerkleTreeAccountInfo(latest: boolean = true) {
    if (latest) {
      this.merkleTreeAccountInfo =
        await this.program.account.compressedAccountMerkleTree.fetch(
          this.pdaPublicKey
        );
      return this.merkleTreeAccountInfo;
    }
    return this.merkleTreeAccountInfo;
  }

  getUpdateProofInputs(leafHash: string) {
    if (!this.prover) {
      this.prover = new Prover(this.idl, "./build-circuit");
    }
    const originalTree = [...[this.merkleTree]][0];
    const subTrees = this.merkleTree.filledSubtrees.map((value) =>
      value.toString()
    );
    this.merkleTree.insert(new BN(leafHash));

    const sibling =
      originalTree.nextIndex === 0
        ? zeroValues[0]
        : originalTree.filledSubtrees[0].toString();
    let proofInputs = {
      updatedRoot: this.merkleTree.root.toString(),
      leaf: leafHash,
      subTrees,
      newSubTrees: this.merkleTree.filledSubtrees.map((value) =>
        value.toString()
      ),
      pathIndices:
        originalTree.nextIndex === 0
          ? "0"
          : (originalTree.nextIndex - 1).toString(),
      zeroValues: zeroValues.slice(0, -1),
      sibling,
      subTreeHash: getSubTreeHash(
        subTrees.map((x) => new BN(x)),
        this.poseidon
      ),
      newSubTreeHash: getSubTreeHash(
        this.merkleTree.filledSubtrees,
        this.poseidon
      ),
    };
    this.prover.addProofInputs(proofInputs);
    return proofInputs;
  }

  async generateUpdateProof({
    leafHash,
    proofInputs,
  }: {
    leafHash: string;
    proofInputs?: any;
  }) {
    if (proofInputs) {
      this.prover = new Prover(this.idl, "./build-circuit");
      this.prover.addProofInputs(proofInputs);
    } else if (leafHash) {
      this.getUpdateProofInputs(leafHash);
    } else if (!this.prover) throw new Error("Prover not initialized");

    return this.prover.fullProveAndParse();
  }

  getProgramParameters(insertValue: string) {
    let leafHash = this.poseidon.F.toString(this.poseidon([insertValue]));

    const circuitPath = path.join("build-circuit");

    const programParameters: ProgramParameters = {
      inputs: this.getUpdateProofInputs(leafHash),
      verifierIdl: this.idl,
      path: circuitPath,
      accounts: {
        compressedAccountMerkleTree: this.pdaPublicKey,
      },
      circuitName: "compressedAccountUpdate",
    };
    return programParameters;
  }

  async insertLeaf(insertValue: string) {
    await this.user.getBalance();
    let programParameters = this.getProgramParameters(insertValue);
    return this.user.executeAppUtxo({
      inUtxos: [this.user.getAllUtxos()[0]],
      // addInUtxos: true,
      programParameters,
      action: Action.TRANSFER,
      confirmOptions: ConfirmOptions.spendable,
    });
  }

  getProofInputsInclusionProofInputs(leafInput: string, referenceValue: BN) {
    this.prover = new Prover(this.idl, "./build-circuit", "inclusionProof");
    let leafHash = this.poseidon.F.toString(this.poseidon([leafInput]));
    let index = this.merkleTree.leaves.findIndex(
      (_leafHash) => _leafHash.toString() === leafHash
    );
    if (index === -1) throw new Error("Leaf not found in the merkle tree");
    let fullMerkleTree = new MerkleTree(
      18,
      this.poseidon,
      this.merkleTree.leaves.map((x) => x.toString())
    );
    let proofInputs = {
      leafPreimage: leafInput,
      index,
      root: this.merkleTree.root.toString(),
      referenceValue: referenceValue.toString(),
      pathElements: fullMerkleTree.path(index).pathElements,
    };
    this.prover.addProofInputs(proofInputs);
    return proofInputs;
  }

  async generateInclusionProof({
    leafInput,
    proofInputs,
    referenceValue,
  }: {
    leafInput: string;
    referenceValue: BN;
    proofInputs?: any;
  }) {
    if (proofInputs) {
      this.prover = new Prover(this.idl, "./build-circuit", "inclusionProof");
      proofInputs = this.prover.addProofInputs(proofInputs);
    } else if (leafInput) {
      proofInputs = this.getProofInputsInclusionProofInputs(
        leafInput,
        referenceValue
      );
    } else if (!this.prover) throw new Error("Prover not initialized");
    // TODO: debug and remove
    this.prover.proofInputs = proofInputs;
    return this.prover.fullProveAndParse();
  }

  async verifyInclusionGte({
    leafInput,
    referenceValue,
  }: {
    leafInput: string;
    referenceValue: BN;
  }) {
    if (!this.program) throw new Error("Program not initialized");
    let proof = await this.generateInclusionProof({
      leafInput,
      referenceValue,
    });
    let merkleTreeAccountInfo = await this.getMerkleTreeAccountInfo();
    const rootIndex = merkleTreeAccountInfo.rootHistory.findIndex(
      (root) => root.toString() === proof.parsedPublicInputs[0].toString()
    );
    const ix = await this.program.methods
      .proveInclusionValueGte(
        proof.parsedProof.proofA,
        proof.parsedProof.proofB,
        proof.parsedProof.proofC,
        new BN(rootIndex),
        referenceValue
      )
      .accounts({
        compressedAccountMerkleTree: this.pdaPublicKey,
        signer: this.user.provider.wallet.publicKey,
      })
      .instruction();
    return sendAndConfirmInstructionWithWallet(
      [ix],
      this.user.provider.connection,
      this.user.provider.wallet
    );
  }
}

const sendInstructionWithWallet = async (
  instructions: TransactionInstruction[],
  connection: Connection,
  payer: Wallet
) => {
  let recentBlockhash = await connection.getLatestBlockhash();
  const txMsg = new TransactionMessage({
    payerKey: payer.publicKey,
    instructions: instructions,
    recentBlockhash: recentBlockhash.blockhash,
  });
  let versionedTx = new VersionedTransaction(txMsg.compileToV0Message());
  await payer.signTransaction(versionedTx);

  return connection.sendTransaction(versionedTx, confirmConfig);
};

const sendAndConfirmInstructionWithWallet = async (
  instructions: TransactionInstruction[],
  connection: Connection,
  payer: Wallet
) => {
  let signature = await sendInstructionWithWallet(
    instructions,
    connection,
    payer
  );
  await confirmTransaction(connection, signature, "confirmed");
  return signature;
};
