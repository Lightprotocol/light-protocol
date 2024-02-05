import { BN, BorshAccountsCoder, Idl, utils } from "@coral-xyz/anchor";
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

import {
  AUTHORITY,
  UTXO_PREFIX_LENGTH,
  merkleTreeProgramId,
} from "../constants";

import {
  PspTransactionInput,
  TransactionAccounts,
  PublicTransactionVariables,
  getVerifierConfig,
  getVerifierProgram,
} from "../transaction/psp-transaction";
import {
  Action,
  lightAccounts,
  PublicInputs,
  remainingAccount,
} from "../types";
import { MINT } from "../test-utils";
import { Provider } from "../provider";
import {
  SolanaTransactionError,
  SolanaTransactionErrorCode,
  TransactionError,
  TransactionErrorCode,
} from "../errors";
import {
  createAccountObject,
  firstLetterToLower,
  firstLetterToUpper,
  getNullifierPda,
} from "../utils";

type SolanaInstructionInputs = {
  publicInputs?: PublicInputs;
  rootIndex?: number;
  proofBytes?: any;
  proofBytesApp?: any;
  publicInputsApp?: any;
  encryptedUtxos?: Uint8Array;
  message?: Uint8Array;
  verifierConfig: any;
  ataCreationFee?: boolean;
  action: Action;
  customInputs?: any;
};

export type SolanaRemainingAccounts = {
  nullifierPdaPubkeys?: remainingAccount[];
};

export type SolanaTransactionInputs = {
  action: Action;
  merkleTreeSet: PublicKey;
  publicTransactionVariables: PublicTransactionVariables;
  systemProof: { parsedProof: any; parsedPublicInputsObject: any };
  pspTransactionInput?: PspTransactionInput;
  pspProof?: { parsedProof: any; parsedPublicInputsObject: any };
  rpcRecipientSol?: PublicKey;
  systemPspIdl: Idl;
  rootIndex: number;
  instructionNamePrefix?: string;
  customInputs?: any;
};

export function getSolanaRemainingAccounts(
  systemProofPublicInputs: PublicInputs,
) {
  const nullifiers = systemProofPublicInputs.publicNullifier;
  const remainingAccounts: SolanaRemainingAccounts = {};
  remainingAccounts["nullifierPdaPubkeys"] = [];
  for (const i in nullifiers) {
    remainingAccounts.nullifierPdaPubkeys.push({
      isSigner: false,
      isWritable: true,
      pubkey: getNullifierPda(nullifiers[i], merkleTreeProgramId),
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
  instructionNamePrefix,
  instructionName,
  customInputs,
  removeZeroUtxos,
}: {
  action: Action;
  rootIndex: number;
  systemProof: { parsedProof: any; parsedPublicInputsObject: any };
  remainingSolanaAccounts: SolanaRemainingAccounts;
  accounts: lightAccounts;
  publicTransactionVariables: PublicTransactionVariables;
  systemPspIdl: Idl;
  pspIdl?: Idl;
  pspTransactionInput?: PspTransactionInput;
  pspProof?: { parsedProof: any; parsedPublicInputsObject: any };
  instructionNamePrefix?: string;
  instructionName?: string;
  customInputs?: any;
  removeZeroUtxos?: boolean;
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
    customInputs,
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
  const inUtxoHashes: number[][] = [];
  instructionInputs.publicInputs?.publicInUtxoHash?.map((el) => {
    if (removeZeroUtxos && el.toString() !== new Array(32).fill(0).toString()) {
      inUtxoHashes.push(el);
    }
  });
  let inputObject = {
    ...instructionInputs.customInputs,
    message: instructionInputs.message,
    ...instructionInputs.proofBytes,
    ...instructionInputs.publicInputs,
    rootIndex: new BN(instructionInputs.rootIndex!),
    rootIndexes: instructionInputs.publicInputs?.publicInUtxoHash?.map(
      () => new BN(instructionInputs.rootIndex!),
    ),
    rpcFee: publicTransactionVariables.rpcFee,
    encryptedUtxos: Buffer.from(instructionInputs.encryptedUtxos!),
    inUtxoHashes,
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
   * - for spl decompressions we need an initialized associated token we can decompress to
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

  const instructionNames =
    instructionName !== undefined
      ? [instructionName]
      : getInstructionNamesByPrefix(invokingProgramIdl, instructionNamePrefix); //getOrderedInstructionNames(invokingProgramIdl);
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
    if (i === instructionNames.length - 1 && !instructionName) {
      const remainingAccounts = [
        ...remainingSolanaAccounts!.nullifierPdaPubkeys!,
      ];
      method.remainingAccounts(remainingAccounts);
    }
    if (i === instructionNames.length - 1 && instructionName) {
      method.remainingAccounts(remainingSolanaAccounts as any);
    }

    const ix = await method.instruction();

    instructions.push(ix);
  }
  return instructions;
}

// pspProof, systemProof,pspTransactionInput, txParams
export async function sendAndConfirmCompressTransaction({
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
    merkleTreeSet,
    pspProof,
    pspTransactionInput,
    systemProof,
    rootIndex,
    instructionNamePrefix,
    customInputs,
  } = solanaTransactionInputs;

  const remainingSolanaAccounts = getSolanaRemainingAccounts(
    solanaTransactionInputs.systemProof.parsedPublicInputsObject,
  );
  const accounts = prepareAccounts({
    transactionAccounts: publicTransactionVariables.accounts,
    merkleTreeSet: merkleTreeSet,
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
    instructionNamePrefix,
    customInputs,
  });

  const txHash = await provider.sendAndConfirmSolanaInstructions(instructions);
  const rpcMerkleTreeUpdateResponse = "notPinged";
  return { txHash, response: rpcMerkleTreeUpdateResponse };
}

// pspProof, systemProof,pspTransactionInput, txParams
export async function sendAndConfirmCompressedTransaction({
  provider,
  solanaTransactionInputs,
}: {
  provider: Provider;
  solanaTransactionInputs: SolanaTransactionInputs;
}): Promise<any> {
  const {
    publicTransactionVariables,
    action,
    merkleTreeSet,
    pspProof,
    pspTransactionInput,
    systemProof,
    rpcRecipientSol,
    systemPspIdl,
    rootIndex,
  } = solanaTransactionInputs;
  if (action === Action.COMPRESS) {
    throw new SolanaTransactionError(
      SolanaTransactionErrorCode.INVALID_ACTION,
      "sendAndConfirmCompressedTransaction",
      `Action ${action} is COMPRESS use sendAndConfirmCompressTransaction.`,
    );
  }
  if (!rpcRecipientSol) {
    throw new SolanaTransactionError(
      SolanaTransactionErrorCode.RPC_RECIPIENT_SOL_UNDEFINED,
      "sendAndConfirmCompressedTransaction",
      `Rpc recipient sol is undefined.`,
    );
  }

  const remainingSolanaAccounts = getSolanaRemainingAccounts(
    solanaTransactionInputs.systemProof.parsedPublicInputsObject,
  );
  const accounts = prepareAccounts({
    transactionAccounts: publicTransactionVariables.accounts,
    merkleTreeSet: merkleTreeSet,
    rpcRecipientSol,
    signer: publicTransactionVariables.accounts.rpcPublicKey,
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

  const txHash = await provider.rpc.sendAndConfirmSolanaInstructions(
    instructions,
    provider.provider.connection,
    undefined,
    undefined,
    provider,
  );
  const rpcMerkleTreeUpdateResponse = "notPinged";

  return {
    txHash: { signatures: txHash }, // TODO: unify interface
    response: rpcMerkleTreeUpdateResponse,
  };
}

// TODO: unify event Merkle tree and transaction Merkle tree so that only one is passed
export function prepareAccounts({
  transactionAccounts,
  merkleTreeSet,
  signer,
  rpcRecipientSol,
  verifierState,
}: {
  transactionAccounts: TransactionAccounts;
  merkleTreeSet?: PublicKey;
  signer?: PublicKey;
  rpcRecipientSol?: PublicKey;
  verifierState?: PublicKey;
}): lightAccounts {
  const {
    senderSol,
    senderSpl,
    recipientSol,
    recipientSpl,
    rpcPublicKey,
    pspId,
    systemPspId,
  } = transactionAccounts;
  const verifierProgramId = pspId ? pspId : systemPspId;
  if (!signer) {
    signer = rpcPublicKey;
  }
  if (!verifierState) {
    verifierState = getVerifierStatePda(verifierProgramId, signer);
  }
  const accounts = {
    systemProgramId: SystemProgram.programId,
    tokenProgram: TOKEN_PROGRAM_ID,
    logWrapper: SPL_NOOP_PROGRAM_ID,
    merkleTreeSet: transactionAccounts.merkleTreeSet,
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
    rpcRecipientSol: rpcRecipientSol ? rpcRecipientSol : AUTHORITY,
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
export function getInstructionNamesByPrefix(verifierIdl: Idl, prefix?: string) {
  prefix = prefix ? prefix : "";
  const instructions = verifierIdl.instructions
    .filter((instruction) => {
      if (prefix) {
        instruction.name.startsWith(prefix);
      } else {
        return true;
      }
    })
    .map((instruction) => instruction);
  return orderInstructionNames(instructions);
}
export const orderInstructionNames = (instructionNames: { name: string }[]) => {
  const orderedInstructionNames = instructionNames
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
