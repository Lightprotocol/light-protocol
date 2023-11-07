import { BN, BorshAccountsCoder, Idl, utils } from "@coral-xyz/anchor";
import { TransactionParameters } from "./transactionParameters";
import {
  AUTHORITY,
  UTXO_PREFIX_LENGTH,
  merkleTreeProgramId,
} from "../constants";
import {
  Action,
  ConfirmOptions,
  MINT,
  Provider,
  PspTransactionInput,
  PublicInputs,
  SolanaTransactionError,
  SolanaTransactionErrorCode,
  Transaction,
  TransactionError,
  TransactionErrorCode,
  createAccountObject,
  firstLetterToLower,
  firstLetterToUpper,
  remainingAccount,
} from "../index";
import { createAssociatedTokenAccountInstruction } from "@solana/spl-token";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";

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
      pubkey: Transaction.getNullifierPdaPublicKey(
        nullifiers[i],
        merkleTreeProgramId,
      ),
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
export async function createSolanaInstructions(
  rootIndex: BN,
  systemProof: { parsedProof: any; parsedPublicInputsObject: any },
  remainingSolanaAccounts: SolanaRemainingAccounts,
  txParams: TransactionParameters,
  pspIdl: Idl,
  pspTransactionInput?: PspTransactionInput,
  pspProof?: { parsedProof: any; parsedPublicInputsObject: any },
): Promise<TransactionInstruction[]> {
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
  if (!txParams.encryptedUtxos)
    throw new TransactionError(
      TransactionErrorCode.ENCRYPTED_UTXOS_UNDEFINED,
      "getInstructions",
    );
  const instructionInputs: SolanaInstructionInputs = {
    proofBytes: systemProof.parsedProof,
    publicInputs: systemProof.parsedPublicInputsObject,
    proofBytesApp,
    publicInputsApp,
    rootIndex,
    encryptedUtxos: txParams.encryptedUtxos,
    verifierConfig: txParams.verifierConfig,
    ataCreationFee: txParams.ataCreationFee,
    action: txParams.action,
  };
  const verifierProgram = TransactionParameters.getVerifierProgram(
    pspIdl,
    {} as any,
  );

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
    relayerFee: txParams.relayer.getRelayerFee(
      instructionInputs.ataCreationFee,
    ),
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
    if (!txParams.accounts.recipientSpl)
      throw new TransactionError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getInstructions",
        "Probably sth in the associated token address generation went wrong",
      );
    if (!txParams.accounts.recipientSol)
      throw new TransactionError(
        TransactionErrorCode.SPL_RECIPIENT_UNDEFINED,
        "getInstructions",
        "Probably sth in the associated token address generation went wrong",
      );
    const ix = createAssociatedTokenAccountInstruction(
      txParams.relayer.accounts.relayerPubkey,
      txParams.accounts.recipientSpl,
      txParams.accounts.recipientSol,
      MINT,
    );
    instructions.push(ix);
  }

  const instructionNames = getOrderedInstructionNames(pspIdl);
  for (let i = 0; i < instructionNames.length; i++) {
    const instruction = instructionNames[i];
    const coder = new BorshAccountsCoder(pspIdl);

    const accountName = "instructionData" + firstLetterToUpper(instruction);
    const inputs = createAccountObject(
      inputObject,
      pspIdl.accounts!,
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
      ...txParams.accounts,
      ...txParams.relayer.accounts,
      ...appAccounts,
      relayerRecipientSol:
        instructionInputs.action === Action.SHIELD
          ? AUTHORITY
          : txParams.relayer.accounts.relayerRecipientSol,
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
  transaction: TransactionParameters;
  systemProof: { parsedProof: any; parsedPublicInputsObject: any };
  pspTransactionInput?: PspTransactionInput;
  pspProof?: { parsedProof: any; parsedPublicInputsObject: any };
};

// pspProof, systemProof,pspTransactionInput, txParams
export async function sendAndConfirmShieldTransaction({
  provider,
  solanaTransactionInputs,
  confirmOptions = ConfirmOptions.spendable,
}: {
  provider: Provider;
  solanaTransactionInputs: SolanaTransactionInputs;
  confirmOptions?: ConfirmOptions;
}): Promise<any> {
  if (solanaTransactionInputs.transaction.action !== Action.SHIELD) {
    throw new SolanaTransactionError(
      SolanaTransactionErrorCode.INVALID_ACTION,
      "sendAndConfirmShieldTransaction",
      `Action ${solanaTransactionInputs.transaction.action} is not SHIELD use sendAndConfirmShieldedTransaction.`,
    );
  }
  const { rootIndex, remainingAccounts: remainingMerkleTreeAccounts } =
    await provider.getRootIndex();

  const remainingSolanaAccounts = getSolanaRemainingAccounts(
    solanaTransactionInputs.systemProof.parsedPublicInputsObject,
    remainingMerkleTreeAccounts,
  );

  // createSolanaInstructionsWithAccounts
  const instructions = await createSolanaInstructions(
    rootIndex,
    solanaTransactionInputs.systemProof,
    remainingSolanaAccounts,
    solanaTransactionInputs.transaction,
    solanaTransactionInputs.pspTransactionInput
      ? solanaTransactionInputs.pspTransactionInput.verifierIdl
      : solanaTransactionInputs.transaction.verifierIdl,
    solanaTransactionInputs.pspTransactionInput,
    solanaTransactionInputs.pspProof,
  );

  const txHash = await provider.sendAndConfirmTransaction(instructions);
  let relayerMerkleTreeUpdateResponse = "notPinged";

  if (confirmOptions === ConfirmOptions.finalized) {
    // Don't add await here, because the utxos have been spent the transaction is final.
    // We just want to ping the relayer to update the Merkle tree not wait for the update.
    // This option should be used to speed up transactions when we do not expect a following
    // transaction which depends on the newly created utxos.
    provider.relayer.updateMerkleTree(provider);
    relayerMerkleTreeUpdateResponse = "pinged relayer";
  }

  if (confirmOptions === ConfirmOptions.spendable) {
    await provider.relayer.updateMerkleTree(provider);
    relayerMerkleTreeUpdateResponse = "success";
  }
  return { txHash, response: relayerMerkleTreeUpdateResponse };
}

// pspProof, systemProof,pspTransactionInput, txParams
export async function sendAndConfirmShieldedTransaction({
  provider,
  solanaTransactionInputs,
  confirmOptions = ConfirmOptions.spendable,
}: {
  provider: Provider;
  solanaTransactionInputs: SolanaTransactionInputs;
  confirmOptions?: ConfirmOptions;
}): Promise<any> {
  if (solanaTransactionInputs.transaction.action === Action.SHIELD) {
    throw new SolanaTransactionError(
      SolanaTransactionErrorCode.INVALID_ACTION,
      "sendAndConfirmShieldedTransaction",
      `Action ${solanaTransactionInputs.transaction.action} is SHIELD use sendAndConfirmShieldTransaction.`,
    );
  }
  const { rootIndex, remainingAccounts: remainingMerkleTreeAccounts } =
    await provider.getRootIndex();

  const remainingSolanaAccounts = getSolanaRemainingAccounts(
    solanaTransactionInputs.systemProof.parsedPublicInputsObject,
    remainingMerkleTreeAccounts,
  );

  // createSolanaInstructionsWithAccounts
  const instructions = await createSolanaInstructions(
    rootIndex,
    solanaTransactionInputs.systemProof,
    remainingSolanaAccounts,
    solanaTransactionInputs.transaction,
    solanaTransactionInputs.pspTransactionInput
      ? solanaTransactionInputs.pspTransactionInput.verifierIdl
      : solanaTransactionInputs.transaction.verifierIdl,
    solanaTransactionInputs.pspTransactionInput,
    solanaTransactionInputs.pspProof,
  );

  const txHash = await provider.sendAndConfirmShieldedTransaction(instructions);
  let relayerMerkleTreeUpdateResponse = "notPinged";

  if (confirmOptions === ConfirmOptions.finalized) {
    // Don't add await here, because the utxos have been spent the transaction is final.
    // We just want to ping the relayer to update the Merkle tree not wait for the update.
    // This option should be used to speed up transactions when we do not expect a following
    // transaction which depends on the newly created utxos.
    provider.relayer.updateMerkleTree(provider);
    relayerMerkleTreeUpdateResponse = "pinged relayer";
  }

  if (confirmOptions === ConfirmOptions.spendable) {
    await provider.relayer.updateMerkleTree(provider);
    relayerMerkleTreeUpdateResponse = "success";
  }
  return { txHash, response: relayerMerkleTreeUpdateResponse };
}
