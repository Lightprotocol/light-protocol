/**
 * SPL Token test utilities for LiteSVM
 * Provides helper functions that work directly with LiteSVM for testing SPL token operations
 */

import {
  PublicKey,
  Transaction,
  SystemProgram,
  Signer,
  Keypair,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  MINT_SIZE,
  getAssociatedTokenAddressSync,
  createInitializeMint2Instruction,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  createTransferCheckedInstruction,
  getMinimumBalanceForRentExemptMint,
  AccountLayout,
  MintLayout,
} from '@solana/spl-token';
import { LiteSVMRpc } from './litesvm-rpc';

/**
 * Create a new SPL token mint using LiteSVM
 */
export async function splCreateMint(
  rpc: LiteSVMRpc,
  payer: Signer,
  mintAuthority: PublicKey,
  freezeAuthority: PublicKey | null,
  decimals: number,
  keypair = Keypair.generate(),
  programId = TOKEN_PROGRAM_ID,
): Promise<PublicKey> {
  const lamports = await getMinimumBalanceForRentExemptMint(rpc);

  const transaction = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: keypair.publicKey,
      space: MINT_SIZE,
      lamports,
      programId,
    }),
    createInitializeMint2Instruction(
      keypair.publicKey,
      decimals,
      mintAuthority,
      freezeAuthority,
      programId,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer, keypair);

  // Send transaction using LiteSVM
  await rpc.sendTransaction(transaction);

  return keypair.publicKey;
}

/**
 * Create an associated token account using LiteSVM
 */
export async function splCreateAssociatedTokenAccount(
  rpc: LiteSVMRpc,
  payer: Signer,
  mint: PublicKey,
  owner: PublicKey,
  programId = TOKEN_PROGRAM_ID,
): Promise<PublicKey> {
  const associatedToken = getAssociatedTokenAddressSync(
    mint,
    owner,
    false,
    programId,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  const transaction = new Transaction().add(
    createAssociatedTokenAccountInstruction(
      payer.publicKey,
      associatedToken,
      owner,
      mint,
      programId,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer);

  // Send transaction using LiteSVM
  await rpc.sendTransaction(transaction);

  return associatedToken;
}

/**
 * Mint tokens to an account using LiteSVM
 */
export async function splMintTo(
  rpc: LiteSVMRpc,
  payer: Signer,
  mint: PublicKey,
  destination: PublicKey,
  authority: Signer,
  amount: number | bigint,
  programId = TOKEN_PROGRAM_ID,
): Promise<string> {
  const transaction = new Transaction().add(
    createMintToInstruction(
      mint,
      destination,
      authority.publicKey,
      amount,
      [],
      programId,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer, authority);

  // Send transaction using LiteSVM
  return rpc.sendTransaction(transaction);
}

/**
 * Transfer tokens between accounts using LiteSVM
 */
export async function splTransfer(
  rpc: LiteSVMRpc,
  payer: Signer,
  source: PublicKey,
  mint: PublicKey,
  destination: PublicKey,
  owner: Signer,
  amount: number | bigint,
  decimals: number,
  programId = TOKEN_PROGRAM_ID,
): Promise<string> {
  const transaction = new Transaction().add(
    createTransferCheckedInstruction(
      source,
      mint,
      destination,
      owner.publicKey,
      amount,
      decimals,
      [],
      programId,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer, owner);

  // Send transaction using LiteSVM
  return rpc.sendTransaction(transaction);
}

/**
 * Get token account balance
 */
export async function splGetTokenAccountBalance(
  rpc: LiteSVMRpc,
  tokenAccount: PublicKey,
): Promise<bigint> {
  const accountInfo = await rpc.getAccountInfo(tokenAccount);

  if (!accountInfo) {
    throw new Error('Token account not found');
  }

  const data = AccountLayout.decode(accountInfo.data);
  return data.amount;
}

/**
 * Get mint info
 */
export async function splGetMintInfo(
  rpc: LiteSVMRpc,
  mint: PublicKey,
): Promise<{
  mintAuthority: PublicKey | null;
  supply: bigint;
  decimals: number;
  isInitialized: boolean;
  freezeAuthority: PublicKey | null;
}> {
  const accountInfo = await rpc.getAccountInfo(mint);

  if (!accountInfo) {
    throw new Error('Mint not found');
  }

  const data = MintLayout.decode(accountInfo.data);

  return {
    mintAuthority: data.mintAuthorityOption === 0 ? null : new PublicKey(data.mintAuthority),
    supply: data.supply,
    decimals: data.decimals,
    isInitialized: data.isInitialized,
    freezeAuthority: data.freezeAuthorityOption === 0 ? null : new PublicKey(data.freezeAuthority),
  };
}

/**
 * Check if a token account exists
 */
export async function splTokenAccountExists(
  rpc: LiteSVMRpc,
  tokenAccount: PublicKey,
): Promise<boolean> {
  const accountInfo = await rpc.getAccountInfo(tokenAccount);
  return accountInfo !== null;
}