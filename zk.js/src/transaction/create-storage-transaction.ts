import { BN } from "@coral-xyz/anchor";
import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  TransactionSignature,
} from "@solana/web3.js";
import { Action, AppUtxoConfig } from "../types";
import {
  CompressTransaction,
  Transaction,
  DecompressTransaction,
  createCompressTransaction,
  createSystemProofInputs,
  getSystemProof,
  getSystemPspIdl,
  getVerifierProgramId,
  syncInputUtxosMerkleProofs,
  SystemProofInputs,
} from "./psp-transaction";
import { LightWasm } from "@lightprotocol/account.rs";
import { Rpc } from "../rpc";
import {
  CreateUtxoErrorCode,
  TransactionParametersError,
  TransactionParametersErrorCode,
  UserError,
  UserErrorCode,
} from "../errors";
import { MerkleTreeConfig } from "../merkle-tree";
import { Account } from "../account";
import {
  BN254,
  PlaceHolderTData,
  ProgramOutUtxo,
  createProgramOutUtxo,
  createDataHashWithDefaultHashingSchema,
  encryptProgramOutUtxo,
} from "../utxo";
import {
  BN_0,
  MAX_MESSAGE_SIZE,
  MERKLE_TREE_SET,
  TOKEN_PUBKEY_SYMBOL,
  TOKEN_REGISTRY,
} from "../constants";
import {
  createSolanaInstructions,
  getSolanaRemainingAccounts,
  prepareAccounts,
} from "./solana-transaction";
import { Provider } from "../provider";
import { IDL_LIGHT_PSP2IN2OUT_STORAGE } from "../idls";

export async function prepareStoreProgramUtxo({
  token,
  amountSol,
  amountSpl,
  senderTokenAccount,
  recipientPublicKey,
  appUtxo,
  appUtxoConfig,
  account,
  lightWasm,
  assetLookupTable,
}: {
  token?: string;
  amountSol?: BN;
  amountSpl?: BN;
  senderTokenAccount?: PublicKey;
  recipientPublicKey?: string;
  appUtxo?: ProgramOutUtxo<PlaceHolderTData>;
  appUtxoConfig?: AppUtxoConfig;
  account: Account;
  lightWasm: LightWasm;
  assetLookupTable: string[];
}) {
  if (!appUtxo) {
    if (appUtxoConfig) {
      if (!token)
        throw new UserError(
          UserErrorCode.TOKEN_UNDEFINED,
          "prepareStoreProgramUtxo",
        );
      if (!amountSol)
        throw new UserError(
          CreateUtxoErrorCode.PUBLIC_SOL_AMOUNT_UNDEFINED,
          "prepareStoreProgramUtxo",
        );
      if (!amountSpl)
        throw new UserError(
          CreateUtxoErrorCode.PUBLIC_SPL_AMOUNT_UNDEFINED,
          "prepareStoreProgramUtxo",
        );
      const tokenCtx = TOKEN_REGISTRY.get(token);
      if (!tokenCtx)
        throw new UserError(
          UserErrorCode.INVALID_TOKEN,
          "prepareStoreProgramUtxo",
        );
      const recipientAccount = recipientPublicKey
        ? Account.fromPubkey(recipientPublicKey, lightWasm)
        : undefined;

      const dataHash: BN254 = createDataHashWithDefaultHashingSchema(
        appUtxoConfig.appData,
        lightWasm,
      );

      appUtxo = createProgramOutUtxo({
        lightWasm,
        amounts: [amountSol, amountSpl],
        assets: [SystemProgram.programId, tokenCtx.mint],
        encryptionPublicKey: recipientAccount
          ? recipientAccount.encryptionKeypair.publicKey
          : undefined,
        owner: appUtxoConfig.verifierAddress,
        ownerIdl: appUtxoConfig.idl,
        data: appUtxoConfig.appData,
        type: "appUtxo", // TODO: make dynamic
        dataHash,
      });
    } else {
      throw new UserError(
        UserErrorCode.APP_UTXO_UNDEFINED,
        "prepareStoreProgramUtxo",
        "invalid parameters to generate app utxo",
      );
    }
  }
  if (!appUtxo)
    throw new UserError(
      UserErrorCode.APP_UTXO_UNDEFINED,
      "prepareStoreProgramUtxo",
      `app utxo is undefined or could not generate one from provided parameters`,
    );

  if (!token) {
    const utxoAsset =
      appUtxo.amounts[1].toString() === "0"
        ? new PublicKey(0).toBase58()
        : appUtxo.assets[1].toBase58();
    token = TOKEN_PUBKEY_SYMBOL.get(utxoAsset);
  }

  if (!token)
    throw new UserError(
      UserErrorCode.TOKEN_UNDEFINED,
      "prepareStoreProgramUtxo",
    );

  const message = Buffer.from(
    await encryptProgramOutUtxo({
      lightWasm,
      merkleTreePdaPublicKey: MERKLE_TREE_SET,
      compressed: false,
      account,
      utxo: appUtxo,
      assetLookupTable,
    }),
  );

  if (message.length > MAX_MESSAGE_SIZE)
    throw new UserError(
      UserErrorCode.MAX_STORAGE_MESSAGE_SIZE_EXCEEDED,
      "storeData",
      `${message.length}/${MAX_MESSAGE_SIZE}`,
    );

  if (!amountSpl)
    amountSpl =
      appUtxo.amounts[1].toString() === "0" ? undefined : appUtxo.amounts[1];

  const tokenCtx = getTokenContext(token);

  return {
    tokenCtx,
    utxo: appUtxo,
    publicAmountSpl: amountSpl,
    userSplAccount: senderTokenAccount,
    verifierIdl: IDL_LIGHT_PSP2IN2OUT_STORAGE,
    message,
  };
}

export async function compressProgramUtxo({
  token,
  amountSol,
  amountSpl,
  senderTokenAccount,
  recipientPublicKey,
  appUtxo,
  appUtxoConfig,
  account,
  provider,
}: {
  token?: string;
  amountSol?: BN;
  amountSpl?: BN;
  senderTokenAccount?: PublicKey;
  recipientPublicKey?: string;
  appUtxo?: ProgramOutUtxo<PlaceHolderTData>;
  appUtxoConfig?: AppUtxoConfig;
  account: Account;
  provider: Provider;
}): Promise<TransactionSignature[]> {
  const {
    tokenCtx,
    utxo,
    publicAmountSpl,
    userSplAccount,
    verifierIdl,
    message,
  } = await prepareStoreProgramUtxo({
    token,
    amountSol,
    amountSpl,
    senderTokenAccount,
    recipientPublicKey,
    appUtxo,
    appUtxoConfig,
    account,
    assetLookupTable: provider.lookUpTables.assetLookupTable,
    lightWasm: provider.lightWasm,
  });

  const transaction = await createCompressTransaction<
    any,
    ProgramOutUtxo<PlaceHolderTData>
  >({
    message,
    merkleTreeSetPubkey: MERKLE_TREE_SET,
    mint:
      publicAmountSpl && !publicAmountSpl.eq(BN_0) ? tokenCtx.mint : undefined,
    senderSpl: userSplAccount,
    outputUtxos: [utxo],
    signer: provider.wallet.publicKey,
    systemPspId: getVerifierProgramId(verifierIdl),
    account,
    lightWasm: provider.lightWasm,
  });

  const instructions = await proveAndCreateInstructions({
    transaction,
    rpc: provider.rpc,
    account,
    lightWasm: provider.lightWasm,
  });
  const txResult =
    await provider.sendAndConfirmSolanaInstructions(instructions);
  return txResult;
}

export async function proveAndCreateInstructions({
  transaction,
  rpc,
  account,
  lightWasm,
}: {
  transaction: CompressTransaction | Transaction | DecompressTransaction;
  rpc: Rpc;
  account: Account;
  lightWasm: LightWasm;
}): Promise<TransactionInstruction[]> {
  if (!transaction)
    throw new UserError(
      UserErrorCode.TRANSACTION_PARAMETERS_UNDEFINED,
      "compileAndProveTransaction",
      "The method 'createCompressTransactionParameters' must be executed first to generate the parameters that can be compiled and proven.",
    );
  let root: string | undefined = undefined;
  let rootIndex: number | undefined = undefined;
  if (
    transaction.private.inputUtxos &&
    transaction.private.inputUtxos.length != 0
  ) {
    const {
      syncedUtxos,
      root: fetchedRoot,
      index,
    } = await syncInputUtxosMerkleProofs({
      inputUtxos: transaction.private.inputUtxos,
      rpc,
      merkleTreeSet: MERKLE_TREE_SET,
    });
    transaction.private.inputUtxos = syncedUtxos;
    root = fetchedRoot;
    rootIndex = index;
  } else {
    const res = (await rpc.getMerkleRoot(MERKLE_TREE_SET))!;
    root = res.root;
    rootIndex = res.index;
  }
  if (!root) {
    throw new TransactionParametersError(
      TransactionParametersErrorCode.FETCHING_ROOT_FAILED,
      "getTxParams",
      "Fetching root from rpc failed.",
    );
  }
  const systemProofInputs: SystemProofInputs = createSystemProofInputs({
    transaction: transaction,
    root,
    account,
    lightWasm,
  });

  const systemProof = await getSystemProof({
    account: account,
    inputUtxos: transaction.private.inputUtxos,
    verifierIdl: getSystemPspIdl(transaction.public.accounts.systemPspId)!,
    systemProofInputs,
  });

  const remainingSolanaAccounts = getSolanaRemainingAccounts(
    systemProof.parsedPublicInputsObject,
    // TODO: readd remainingMerkleTreeAccounts,
  );
  const accounts = prepareAccounts({
    transactionAccounts: transaction.public.accounts,
    merkleTreeSet: MERKLE_TREE_SET,
    rpcRecipientSol: rpc.accounts.rpcRecipientSol,
    signer: transaction.public.accounts.rpcPublicKey,
  });

  const instructions = await createSolanaInstructions({
    action: transaction["action"] ?? Action.TRANSFER,
    systemProof,
    remainingSolanaAccounts,
    accounts,
    publicTransactionVariables: transaction.public,
    systemPspIdl: getSystemPspIdl(transaction.public.accounts.systemPspId),
    rootIndex,
  });
  return instructions;
}

export const getTokenContext = (token: string) => {
  const tokenCtx = TOKEN_REGISTRY.get(token);
  if (!tokenCtx) {
    throw new UserError(UserErrorCode.INVALID_TOKEN, "prepareStoreProgramUtxo");
  }
  return tokenCtx;
};
