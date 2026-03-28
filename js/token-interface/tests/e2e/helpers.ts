import { AccountState } from "@solana/spl-token";
import {
  Keypair,
  PublicKey,
  Signer,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  Rpc,
  TreeInfo,
  VERSION,
  bn,
  buildAndSignTx,
  createRpc,
  featureFlags,
  newAccountWithLamports,
  selectStateTreeInfo,
  sendAndConfirmTx,
} from "@lightprotocol/stateless.js";
import { createMint, mintTo } from "@lightprotocol/compressed-token";
import { parseLightTokenHot } from "../../src/read";
import { getSplInterfaces } from "../../src/spl-interface";

featureFlags.version = VERSION.V2;

export const TEST_TOKEN_DECIMALS = 9;

export interface MintFixture {
  rpc: Rpc;
  payer: Signer;
  mint: PublicKey;
  mintAuthority: Keypair;
  stateTreeInfo: TreeInfo;
  tokenPoolInfos: Awaited<ReturnType<typeof getSplInterfaces>>;
  freezeAuthority?: Keypair;
}

export async function createMintFixture(options?: {
  withFreezeAuthority?: boolean;
  payerLamports?: number;
}): Promise<MintFixture> {
  const rpc = createRpc();
  const payer = await newAccountWithLamports(
    rpc,
    options?.payerLamports ?? 20e9,
  );
  const mintAuthority = Keypair.generate();
  const mintKeypair = Keypair.generate();
  const freezeAuthority = options?.withFreezeAuthority
    ? Keypair.generate()
    : undefined;

  const mint = (
    await createMint(
      rpc,
      payer,
      mintAuthority.publicKey,
      TEST_TOKEN_DECIMALS,
      mintKeypair,
      undefined,
      undefined,
      freezeAuthority?.publicKey ?? null,
    )
  ).mint;

  const stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
  const tokenPoolInfos = await getSplInterfaces(rpc, mint);

  return {
    rpc,
    payer,
    mint,
    mintAuthority,
    stateTreeInfo,
    tokenPoolInfos,
    freezeAuthority,
  };
}

export async function mintCompressedToOwner(
  fixture: MintFixture,
  owner: PublicKey,
  amount: bigint,
): Promise<void> {
  const selectedSplInterfaceInfo = fixture.tokenPoolInfos.find(
    (info) => info.isInitialized,
  );
  if (!selectedSplInterfaceInfo) {
    throw new Error("No initialized SPL interface info found.");
  }

  const selectedSplInterfaceForMintTo = {
    ...selectedSplInterfaceInfo,
    splInterfacePda: selectedSplInterfaceInfo.poolPda,
    tokenProgram: selectedSplInterfaceInfo.tokenProgramId,
    poolIndex: selectedSplInterfaceInfo.derivationIndex,
  };

  await mintTo(
    fixture.rpc,
    fixture.payer,
    fixture.mint,
    owner,
    fixture.mintAuthority,
    bn(amount.toString()),
    fixture.stateTreeInfo,
    selectedSplInterfaceForMintTo,
  );
}

export async function mintMultipleColdAccounts(
  fixture: MintFixture,
  owner: PublicKey,
  count: number,
  amountPerAccount: bigint,
): Promise<void> {
  for (let i = 0; i < count; i += 1) {
    await mintCompressedToOwner(fixture, owner, amountPerAccount);
  }
}

export async function sendInstructions(
  rpc: Rpc,
  payer: Signer,
  instructions: TransactionInstruction[],
  additionalSigners: Signer[] = [],
): Promise<string> {
  const { blockhash } = await rpc.getLatestBlockhash();
  const tx = buildAndSignTx(instructions, payer, blockhash, additionalSigners);
  return sendAndConfirmTx(rpc, tx);
}

export async function getHotBalance(
  rpc: Rpc,
  tokenAccount: PublicKey,
): Promise<bigint> {
  const info = await rpc.getAccountInfo(tokenAccount);
  if (!info) {
    return BigInt(0);
  }

  return parseLightTokenHot(tokenAccount, info).parsed.amount;
}

export async function getHotDelegate(
  rpc: Rpc,
  tokenAccount: PublicKey,
): Promise<{ delegate: PublicKey | null; delegatedAmount: bigint }> {
  const info = await rpc.getAccountInfo(tokenAccount);
  if (!info) {
    return { delegate: null, delegatedAmount: BigInt(0) };
  }

  const { parsed } = parseLightTokenHot(tokenAccount, info);
  return {
    delegate: parsed.delegate,
    delegatedAmount: parsed.delegatedAmount ?? BigInt(0),
  };
}

export async function getHotState(
  rpc: Rpc,
  tokenAccount: PublicKey,
): Promise<AccountState> {
  const info = await rpc.getAccountInfo(tokenAccount);
  if (!info) {
    throw new Error(`Account not found: ${tokenAccount.toBase58()}`);
  }

  const { parsed } = parseLightTokenHot(tokenAccount, info);
  return parsed.isFrozen
    ? AccountState.Frozen
    : parsed.isInitialized
      ? AccountState.Initialized
      : AccountState.Uninitialized;
}

export async function getCompressedAmounts(
  rpc: Rpc,
  owner: PublicKey,
  mint: PublicKey,
): Promise<bigint[]> {
  const result = await rpc.getCompressedTokenAccountsByOwner(owner, { mint });

  return result.items
    .map((account) => BigInt(account.parsed.amount.toString()))
    .sort((left, right) => {
      if (right > left) {
        return 1;
      }

      if (right < left) {
        return -1;
      }

      return 0;
    });
}
