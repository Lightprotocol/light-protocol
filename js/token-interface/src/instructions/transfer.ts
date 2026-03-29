import { Buffer } from "buffer";
import { SystemProgram, TransactionInstruction } from "@solana/web3.js";
import { LIGHT_TOKEN_PROGRAM_ID } from "@lightprotocol/stateless.js";
import { getSplInterfaces } from "../spl-interface";
import { createUnwrapInstruction } from "./unwrap";
import { getMintDecimals, toBigIntAmount } from "../helpers";
import { getAtaAddress } from "../read";
import type {
  CreateRawTransferInstructionInput,
  CreateTransferInstructionsInput,
} from "../types";
import { createLoadInstructions } from "./load";
import { toInstructionPlan } from "./_plan";
import { createAtaInstruction } from "./ata";

const LIGHT_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR = 12;

export function createTransferCheckedInstruction({
  source,
  destination,
  mint,
  authority,
  payer,
  amount,
  decimals,
}: CreateRawTransferInstructionInput): TransactionInstruction {
  const data = Buffer.alloc(10);
  data.writeUInt8(LIGHT_TOKEN_TRANSFER_CHECKED_DISCRIMINATOR, 0);
  data.writeBigUInt64LE(BigInt(amount), 1);
  data.writeUInt8(decimals, 9);

  return new TransactionInstruction({
    programId: LIGHT_TOKEN_PROGRAM_ID,
    keys: [
      { pubkey: source, isSigner: false, isWritable: true },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: destination, isSigner: false, isWritable: true },
      {
        pubkey: authority,
        isSigner: true,
        isWritable: payer.equals(authority),
      },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      {
        pubkey: payer,
        isSigner: !payer.equals(authority),
        isWritable: true,
      },
    ],
    data,
  });
}

/**
 * Canonical web3.js transfer flow builder.
 * Returns an instruction array for a single transfer flow (setup + transfer).
 */
export async function buildTransferInstructions({
  rpc,
  payer,
  mint,
  sourceOwner,
  authority,
  recipient,
  tokenProgram,
  amount,
}: CreateTransferInstructionsInput): Promise<TransactionInstruction[]> {
  const amountBigInt = toBigIntAmount(amount);
  const recipientTokenProgramId = tokenProgram ?? LIGHT_TOKEN_PROGRAM_ID;
  const decimals = await getMintDecimals(rpc, mint);
  const transferSplInterfaces = recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)
    ? undefined
    : await getSplInterfaces(rpc, mint);
  const senderLoadInstructions = await createLoadInstructions({
    rpc,
    payer,
    owner: sourceOwner,
    mint,
    authority,
    wrap: true,
    decimals,
    splInterfaces: transferSplInterfaces,
  });
  const recipientAta = getAtaAddress({
    owner: recipient,
    mint,
    programId: recipientTokenProgramId,
  });
  const recipientLoadInstructions: TransactionInstruction[] = [];
  const senderAta = getAtaAddress({
    owner: sourceOwner,
    mint,
  });
  let transferInstruction: TransactionInstruction;
  if (recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
    transferInstruction = createTransferCheckedInstruction({
      source: senderAta,
      destination: recipientAta,
      mint,
      authority,
      payer,
      amount: amountBigInt,
      decimals,
    });
  } else {
    const splInterface = transferSplInterfaces!.find(
      (info) =>
        info.isInitialized && info.tokenProgramId.equals(recipientTokenProgramId),
    );
    if (!splInterface) {
      throw new Error(
        `No initialized SPL pool found for tokenProgram ${recipientTokenProgramId.toBase58()}.`,
      );
    }
    transferInstruction = createUnwrapInstruction({
      source: senderAta,
      destination: recipientAta,
      owner: authority,
      mint,
      amount: amountBigInt,
      splInterface,
      decimals,
      payer,
    });
  }

  return [
    ...senderLoadInstructions,
    createAtaInstruction({
      payer,
      owner: recipient,
      mint,
      programId: recipientTokenProgramId,
    }),
    ...recipientLoadInstructions,
    transferInstruction,
  ];
}

/**
 * No-wrap transfer flow builder (advanced).
 */
export async function buildTransferInstructionsNowrap({
  rpc,
  payer,
  mint,
  sourceOwner,
  authority,
  recipient,
  tokenProgram,
  amount,
}: CreateTransferInstructionsInput): Promise<TransactionInstruction[]> {
  const amountBigInt = toBigIntAmount(amount);
  const recipientTokenProgramId = tokenProgram ?? LIGHT_TOKEN_PROGRAM_ID;
  const decimals = await getMintDecimals(rpc, mint);
  const transferSplInterfaces = recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)
    ? undefined
    : await getSplInterfaces(rpc, mint);
  const senderLoadInstructions = await createLoadInstructions({
    rpc,
    payer,
    owner: sourceOwner,
    mint,
    authority,
    wrap: false,
    decimals,
    splInterfaces: transferSplInterfaces,
  });
  const recipientAta = getAtaAddress({
    owner: recipient,
    mint,
    programId: recipientTokenProgramId,
  });
  const senderAta = getAtaAddress({
    owner: sourceOwner,
    mint,
  });

  let transferInstruction: TransactionInstruction;
  if (recipientTokenProgramId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
    transferInstruction = createTransferCheckedInstruction({
      source: senderAta,
      destination: recipientAta,
      mint,
      authority,
      payer,
      amount: amountBigInt,
      decimals,
    });
  } else {
    const splInterface = transferSplInterfaces!.find(
      (info) =>
        info.isInitialized && info.tokenProgramId.equals(recipientTokenProgramId),
    );
    if (!splInterface) {
      throw new Error(
        `No initialized SPL pool found for tokenProgram ${recipientTokenProgramId.toBase58()}.`,
      );
    }
    transferInstruction = createUnwrapInstruction({
      source: senderAta,
      destination: recipientAta,
      owner: authority,
      mint,
      amount: amountBigInt,
      splInterface,
      decimals,
      payer,
    });
  }

  return [...senderLoadInstructions, transferInstruction];
}

export async function createTransferInstructionPlan(
  input: CreateTransferInstructionsInput,
) {
  return toInstructionPlan(await buildTransferInstructions(input));
}

export { buildTransferInstructions as createTransferInstructions };
