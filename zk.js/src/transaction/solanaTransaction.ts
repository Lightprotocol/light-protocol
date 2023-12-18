import { BN, BorshAccountsCoder, Idl, utils } from "@coral-xyz/anchor";
import {
  AUTHORITY,
  UTXO_PREFIX_LENGTH,
  merkleTreeProgramId,
} from "../constants";
import {
  Action,
  MINT,
  Provider,
  PspTransactionInput,
  PublicInputs,
  PublicTransactionVariables,
  SolanaTransactionError,
  SolanaTransactionErrorCode,
  TransactionAccounts,
  TransactionError,
  TransactionErrorCode,
  createAccountObject,
  firstLetterToLower,
  firstLetterToUpper,
  getVerifierConfig,
  getVerifierProgram,
  lightAccounts,
  remainingAccount,
} from "../index";
import {
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
} from "@solana/spl-token";
import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";

type SolanaInstructionInputs = {
  publicInputs?: PublicInputs;
  rootIndex?: BN;
  proofBytes?: any;
  proofBytesApp?: any;
  publicInputsApp?: any;
  encryptedUtxos?: Uint8Array;
  message?: Uint8Array;
  verifierConfig: any;
  ataCreationFee?: boolean;
  action: Action;
};

export type SolanaRemainingAccounts = {
  nullifierPdaPubkeys?: remainingAccount[];
  leavesPdaPubkeys?: remainingAccount[];
  nextTransactionMerkleTree?: remainingAccount;
  nextEventMerkleTree?: remainingAccount;
};

export function getSolanaRemainingAccounts(
  systemProofPublicInputs: PublicInputs,
  remainingMerkleTreeAccounts?: {
    nextTransactionMerkleTree?: remainingAccount;
    nextEventMerkleTree?: remainingAccount;
  },
) {
  const nullifiers = systemProofPublicInputs.inputNullifier;
  const remainingAccounts: SolanaRemainingAccounts = {
    ...remainingMerkleTreeAccounts,
  };
  remainingAccounts["nullifierPdaPubkeys"] = [];
  for (const i in nullifiers) {
    remainingAccounts.nullifierPdaPubkeys.push({
      isSigner: false,
      isWritable: true,
      pubkey: getNullifierPda(nullifiers[i], merkleTreeProgramId),
    });
  }

  remainingAccounts["leavesPdaPubkeys"] = [];

  for (let j = 0; j < systemProofPublicInputs.outputCommitment.length; j += 2) {
    remainingAccounts.leavesPdaPubkeys.push({
      isSigner: false,
      isWritable: true,
      pubkey: PublicKey.findProgramAddressSync(
        [
          Buffer.from(Array.from(systemProofPublicInputs.outputCommitment[j])),
          utils.bytes.utf8.encode("leaves"),
        ],
        merkleTreeProgramId,
      )[0],
    });
  }
  return remainingAccounts;
}

export function getVerifierStatePda(
  verifierProgramId: PublicKey,
  signerPublicKey: PublicKey,
) {
  return PublicKey.findProgramAddressSync(
    [signerPublicKey.toBytes(), utils.bytes.utf8.encode("VERIFIER_STATE")],
    verifierProgramId,
  )[0];
}

// TODO: create solanaInstructions.ts file, to separate all solana logic from other logic
// TODO: make getSolanaInstructionAccounts function
export async function createSolanaInstructions({
  rootIndex,
  systemProof,
  remainingSolanaAccounts,
  accounts,
  publicTransactionVariables,
  action,
  pspTransactionInput,
  pspProof,
  pspIdl,
  systemPspIdl,
}: {
  action: Action;
  rootIndex: BN;
  systemProof: { parsedProof: any; parsedPublicInputsObject: any };
  remainingSolanaAccounts: SolanaRemainingAccounts;
  accounts: lightAccounts;
  publicTransactionVariables: PublicTransactionVariables;
  systemPspIdl: Idl;
  pspIdl?: Idl;
  pspTransactionInput?: PspTransactionInput;
  pspProof?: { parsedProof: any; parsedPublicInputsObject: any };
}): Promise<TransactionInstruction[]> {
  let proofBytesApp = {};
  let publicInputsApp = undefined;
  if (pspProof) {
    proofBytesApp = {
      proofAApp: pspProof.parsedProof.proofA,
      proofBApp: pspProof.parsedProof.proofB,
      proofCApp: pspProof.parsedProof.proofC,
    };
    publicInputsApp = pspProof.parsedPublicInputsObject;
  }
  if (!publicTransactionVariables.encryptedUtxos)
    throw new TransactionError(
      TransactionErrorCode.ENCRYPTED_UTXOS_UNDEFINED,
      "getInstructions",
    );
  const verifierConfig = getVerifierConfig(systemPspIdl);
  const invokingProgramIdl = pspIdl ? pspIdl : systemPspIdl;
  const verifierProgram = getVerifierProgram(invokingProgramIdl, {} as any);
  const instructionInputs: SolanaInstructionInputs = {
    proofBytes: systemProof.parsedProof,
    publicInputs: systemProof.parsedPublicInputsObject,
    proofBytesApp,
    publicInputsApp,
    rootIndex,
    encryptedUtxos: publicTransactionVariables.encryptedUtxos,
    verifierConfig: verifierConfig,
    ataCreationFee: publicTransactionVariables.ataCreationFee,
    action: action,
    message: publicTransactionVariables.message,
  };

  if (!instructionInputs.publicInputs)
    throw new TransactionError(
      TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
      "getInstructions",
    );
  if (!verifierProgram)
    throw new TransactionError(
      TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
      "getInstructions",
    );

  const getOrderedInstructionNames = (verifierIdl: Idl) => {
    const orderedInstructionNames = verifierIdl.instructions
      .filter((instruction) =>
        /First|Second|Third|Fourth|Fifth|Sixth|Seventh|Eighth|Ninth/.test(
          instruction.name,
        ),
      )
      .sort((a, b) => {
        const suffixes = [
          "First",
          "Second",
          "Third",
          "Fourth",
          "Fifth",
          "Sixth",
          "Seventh",
          "Eighth",
          "Ninth",
        ];
        const aIndex = suffixes.findIndex((suffix) => a.name.endsWith(suffix));
        const bIndex = suffixes.findIndex((suffix) => b.name.endsWith(suffix));

        if (aIndex === 7 || bIndex === 7) {
          throw new Error("Found an instruction with the 'Eighth' suffix.");
        }

        return aIndex - bIndex;
      })
      .map((instruction) => instruction.name);

    return orderedInstructionNames;
  };

  if (
    instructionInputs.verifierConfig.out == 2 &&
    instructionInputs.encryptedUtxos &&
    instructionInputs.encryptedUtxos
      .slice(240 + UTXO_PREFIX_LENGTH * 2)
      .some((el) => el !== 0)
  ) {
    instructionInputs.encryptedUtxos = instructionInputs.encryptedUtxos.slice(
      0,
      240 + UTXO_PREFIX_LENGTH * 2,
    );
  }
  let inputObject = {
    message: instructionInputs.message,
    ...instructionInputs.proofBytes,
    ...instructionInputs.publicInputs,
    rootIndex: instructionInputs.rootIndex,
    relayerFee: publicTransactionVariables.relayerFee,
    encryptedUtxos: Buffer.from(instructionInputs.encryptedUtxos!),
  };
  if (pspTransactionInput) {
    inputObject = {
      ...inputObject,
      ...instructionInputs.proofBytesApp,
      ...instructionInputs.publicInputsApp,
    };
  }

  const instructions: TransactionInstruction[] = [];
  // TODO: make mint dynamic
  /**
   * Problem:
   * - for spl unshields we need an initialized associated token we can unshield to
   * - this transaction needs to be signed by the owner of the associated token account? has it?
   */
  if (instructionInputs.ataCreationFee) {
    if (!accounts.recipientSpl)
      throw new TransactionError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getInstructions",
        "Probably sth in the associated token address generation went wrong",
      );
    if (!accounts.recipientSol)
      throw new TransactionError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getInstructions",
        "Probably sth in the associated token address generation went wrong",
      );
    const ix = createAssociatedTokenAccountInstruction(
      accounts.signingAddress,
      accounts.recipientSpl,
      accounts.recipientSol,
      MINT,
    );
    instructions.push(ix);
  }

  const instructionNames = getOrderedInstructionNames(invokingProgramIdl);
  for (let i = 0; i < instructionNames.length; i++) {
    const instruction = instructionNames[i];
    const coder = new BorshAccountsCoder(invokingProgramIdl);

    const accountName = "instructionData" + firstLetterToUpper(instruction);
    const inputs = createAccountObject(
      inputObject,
      invokingProgramIdl.accounts!,
      accountName,
    );

    const inputsVec = (await coder.encode(accountName, inputs)).subarray(8);
    // TODO: check whether app account names overlap with system account names and throw an error if so
    let appAccounts = {};
    if (pspTransactionInput?.accounts) {
      appAccounts = pspTransactionInput.accounts;
    }
    const methodName = firstLetterToLower(instruction);
    const method = verifierProgram.methods[
      methodName as keyof typeof verifierProgram.methods
    ](inputsVec).accounts({
      ...accounts,
      ...appAccounts,
    });

    // Check if it's the last iteration
    if (i === instructionNames.length - 1) {
      const remainingAccounts = [
        ...remainingSolanaAccounts!.nullifierPdaPubkeys!,
        ...remainingSolanaAccounts!.leavesPdaPubkeys!,
      ];
      if (remainingSolanaAccounts!.nextTransactionMerkleTree !== undefined) {
        remainingAccounts.push(
          remainingSolanaAccounts!.nextTransactionMerkleTree,
        );
      }
      if (remainingSolanaAccounts!.nextEventMerkleTree !== undefined) {
        remainingAccounts.push(remainingSolanaAccounts!.nextEventMerkleTree);
      }
      method.remainingAccounts(remainingAccounts);
    }

    const ix = await method.instruction();

    instructions.push(ix);
  }
  return instructions;
}

export type SolanaTransactionInputs = {
  action: Action;
  eventMerkleTree: PublicKey;
  publicTransactionVariables: PublicTransactionVariables;
  systemProof: { parsedProof: any; parsedPublicInputsObject: any };
  pspTransactionInput?: PspTransactionInput;
  pspProof?: { parsedProof: any; parsedPublicInputsObject: any };
  relayerRecipientSol?: PublicKey;
  systemPspIdl: Idl;
};

// pspProof, systemProof,pspTransactionInput, txParams
export async function sendAndConfirmShieldTransaction({
  provider,
  solanaTransactionInputs,
}: {
  provider: Provider;
  solanaTransactionInputs: SolanaTransactionInputs;
}): Promise<any> {
  const {
    systemPspIdl,
    publicTransactionVariables,
    action,
    eventMerkleTree,
    pspProof,
    pspTransactionInput,
    systemProof,
  } = solanaTransactionInputs;
  const { rootIndex, remainingAccounts: remainingMerkleTreeAccounts } =
    await provider.getRootIndex();

  const remainingSolanaAccounts = getSolanaRemainingAccounts(
    solanaTransactionInputs.systemProof.parsedPublicInputsObject,
    remainingMerkleTreeAccounts,
  );
  const accounts = prepareAccounts({
    transactionAccounts: publicTransactionVariables.accounts,
    eventMerkleTreePubkey: eventMerkleTree,
  });

  // createSolanaInstructionsWithAccounts
  const instructions = await createSolanaInstructions({
    action,
    rootIndex,
    systemProof,
    remainingSolanaAccounts,
    accounts,
    systemPspIdl,
    pspIdl: pspTransactionInput?.verifierIdl,
    pspTransactionInput,
    pspProof,
    publicTransactionVariables,
  });

  const txHash = await provider.sendAndConfirmSolanaInstructions(instructions);
  const relayerMerkleTreeUpdateResponse = "notPinged";
  return { txHash, response: relayerMerkleTreeUpdateResponse };
}

// pspProof, systemProof,pspTransactionInput, txParams
export async function sendAndConfirmShieldedTransaction({
  provider,
  solanaTransactionInputs,
}: {
  provider: Provider;
  solanaTransactionInputs: SolanaTransactionInputs;
}): Promise<any> {
  const {
    publicTransactionVariables,
    action,
    eventMerkleTree,
    pspProof,
    pspTransactionInput,
    systemProof,
    relayerRecipientSol,
    systemPspIdl,
  } = solanaTransactionInputs;
  if (action === Action.SHIELD) {
    throw new SolanaTransactionError(
      SolanaTransactionErrorCode.INVALID_ACTION,
      "sendAndConfirmShieldedTransaction",
      `Action ${action} is SHIELD use sendAndConfirmShieldTransaction.`,
    );
  }
  if (!relayerRecipientSol) {
    throw new SolanaTransactionError(
      SolanaTransactionErrorCode.RELAYER_RECIPIENT_SOL_UNDEFINED,
      "sendAndConfirmShieldedTransaction",
      `Relayer recipient sol is undefined.`,
    );
  }
  const { rootIndex, remainingAccounts: remainingMerkleTreeAccounts } =
    await provider.getRootIndex();

  const remainingSolanaAccounts = getSolanaRemainingAccounts(
    solanaTransactionInputs.systemProof.parsedPublicInputsObject,
    remainingMerkleTreeAccounts,
  );
  const accounts = prepareAccounts({
    transactionAccounts: publicTransactionVariables.accounts,
    eventMerkleTreePubkey: eventMerkleTree,
    relayerRecipientSol,
    signer: publicTransactionVariables.accounts.relayerPublicKey,
  });

  // createSolanaInstructionsWithAccounts
  const instructions = await createSolanaInstructions({
    action,
    rootIndex,
    systemProof,
    remainingSolanaAccounts,
    accounts,
    systemPspIdl,
    pspIdl: pspTransactionInput?.verifierIdl,
    pspTransactionInput,
    pspProof,
    publicTransactionVariables,
  });

  const txHash = await provider.relayer.sendAndConfirmSolanaInstructions(
    instructions,
    provider.connection!,
  );
  const relayerMerkleTreeUpdateResponse = "notPinged";

  return { txHash, response: relayerMerkleTreeUpdateResponse };
}

// TODO: unify event Merkle tree and transaction Merkle tree so that only one is passed
export function prepareAccounts({
  transactionAccounts,
  eventMerkleTreePubkey,
  signer,
  relayerRecipientSol,
  verifierState,
}: {
  transactionAccounts: TransactionAccounts;
  eventMerkleTreePubkey: PublicKey;
  signer?: PublicKey;
  relayerRecipientSol?: PublicKey;
  verifierState?: PublicKey;
}): lightAccounts {
  const {
    senderSol,
    senderSpl,
    recipientSol,
    recipientSpl,
    relayerPublicKey,
    pspId,
    systemPspId,
  } = transactionAccounts;
  const verifierProgramId = pspId ? pspId : systemPspId;
  if (!signer) {
    signer = relayerPublicKey;
  }
  if (!verifierState) {
    verifierState = getVerifierStatePda(verifierProgramId, signer);
  }
  const accounts = {
    systemProgramId: SystemProgram.programId,
    tokenProgram: TOKEN_PROGRAM_ID,
    logWrapper: SPL_NOOP_PROGRAM_ID,
    eventMerkleTree: eventMerkleTreePubkey,
    transactionMerkleTree: transactionAccounts.transactionMerkleTree,
    registeredVerifierPda: getRegisteredVerifierPda(
      merkleTreeProgramId,
      systemPspId,
    ),
    authority: getSignerAuthorityPda(merkleTreeProgramId, systemPspId),
    senderSpl,
    recipientSpl,
    senderSol,
    recipientSol,
    programMerkleTree: merkleTreeProgramId,
    tokenAuthority: getTokenAuthorityPda(),
    verifierProgram: pspId ? systemPspId : undefined,
    signingAddress: signer,
    relayerRecipientSol: relayerRecipientSol ? relayerRecipientSol : AUTHORITY,
    verifierState,
  };
  return accounts;
}

export function getRegisteredVerifierPda(
  merkleTreeProgramId: PublicKey,
  verifierProgramId: PublicKey,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [verifierProgramId.toBytes()],
    merkleTreeProgramId,
  )[0];
}

export function getNullifierPda(
  nullifier: number[],
  merkleTreeProgramId: PublicKey,
) {
  return PublicKey.findProgramAddressSync(
    [Uint8Array.from([...nullifier]), utils.bytes.utf8.encode("nf")],
    merkleTreeProgramId,
  )[0];
}

export function getTokenAuthorityPda(): PublicKey {
  return PublicKey.findProgramAddressSync(
    [utils.bytes.utf8.encode("spl")],
    merkleTreeProgramId,
  )[0];
}

export function getSignerAuthorityPda(
  merkleTreeProgramId: PublicKey,
  verifierProgramId: PublicKey,
) {
  return PublicKey.findProgramAddressSync(
    [merkleTreeProgramId.toBytes()],
    verifierProgramId,
  )[0];
}
