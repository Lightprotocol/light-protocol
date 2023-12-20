import { BN } from "@coral-xyz/anchor";
import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  ShieldTransaction,
  Transaction,
  UnshieldTransaction,
  createShieldTransaction,
  createSystemProofInputs,
  getSystemProof,
  getSystemPspIdl,
  getVerifierProgramId,
  syncInputUtxosMerkleProofs,
} from "./pspTransaction";
import { LightWasm } from "@lightprotocol/account.rs";
import {
  Relayer,
  TransactionParametersError,
  TransactionParametersErrorCode,
  UserError,
  UserErrorCode,
  MerkleTreeConfig,
  Account,
  Action,
  AppUtxoConfig,
  IDL_LIGHT_PSP2IN2OUT_STORAGE,
  MAX_MESSAGE_SIZE,
  TOKEN_PUBKEY_SYMBOL,
  createSolanaInstructions,
  getSolanaRemainingAccounts,
  prepareAccounts,
  Provider,
  TOKEN_REGISTRY,
  CreateUtxoErrorCode,
  BN_0,
  ProgramUtxo,
  createProgramOutUtxo,
  encryptProgramOutUtxo,
  ProgramOutUtxo,
} from "../index";

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
  appUtxo?: ProgramOutUtxo;
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
      appUtxo = createProgramOutUtxo({
        lightWasm,
        amounts: [amountSol, amountSpl],
        assets: [SystemProgram.programId, tokenCtx.mint],
        publicKey: recipientAccount
          ? recipientAccount.keypair.publicKey
          : account.keypair.publicKey,
        encryptionPublicKey: recipientAccount
          ? recipientAccount.encryptionKeypair.publicKey
          : undefined,
        pspId: appUtxoConfig.verifierAddress,
        pspIdl: appUtxoConfig.idl,
        utxoData: appUtxoConfig.appData,
        utxoName: "appUtxo", // TODO: make dynamic
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
      appUtxo.outUtxo.amounts[1].toString() === "0"
        ? new PublicKey(0).toBase58()
        : appUtxo.outUtxo.assets[1].toBase58();
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
      merkleTreePdaPublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
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
  appUtxo.includeUtxoData = false;
  if (!amountSpl)
    amountSpl =
      appUtxo.outUtxo.amounts[1].toString() === "0"
        ? undefined
        : appUtxo.outUtxo.amounts[1];

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

export async function shieldProgramUtxo({
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
  appUtxo?: ProgramOutUtxo;
  appUtxoConfig?: AppUtxoConfig;
  account: Account;
  provider: Provider;
}) {
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

  const transaction = await createShieldTransaction({
    message,
    transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(),
    mint:
      publicAmountSpl && !publicAmountSpl.eq(BN_0) ? tokenCtx.mint : undefined,
    senderSpl: userSplAccount,
    outputUtxos: [utxo.outUtxo],
    signer: provider.wallet.publicKey,
    systemPspId: getVerifierProgramId(verifierIdl),
    account,
    lightWasm: provider.lightWasm,
  });
  const instructions = await proveAndCreateInstructions({
    transaction,
    relayer: provider.relayer,
    account,
    lightWasm: provider.lightWasm,
  });
  const txResult = await provider.sendAndConfirmTransaction(instructions);
  return txResult;
}

export async function proveAndCreateInstructions({
  transaction,
  relayer,
  account,
  lightWasm,
}: {
  transaction: ShieldTransaction | Transaction | UnshieldTransaction;
  relayer: Relayer;
  account: Account;
  lightWasm: LightWasm;
}): Promise<TransactionInstruction[]> {
  if (!transaction)
    throw new UserError(
      UserErrorCode.TRANSACTION_PARAMTERS_UNDEFINED,
      "compileAndProveTransaction",
      "The method 'createShieldTransactionParameters' must be executed first to generate the parameters that can be compiled and proven.",
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
      merkleTreePublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      relayer,
    });
    transaction.private.inputUtxos = syncedUtxos;
    root = fetchedRoot;
    rootIndex = index;
  } else {
    const res = (await relayer.getMerkleRoot(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    ))!;
    root = res.root;
    rootIndex = res.index;
  }
  if (!root) {
    throw new TransactionParametersError(
      TransactionParametersErrorCode.FETCHING_ROOT_FAILED,
      "getTxParams",
      "Fetching root from relayer failed.",
    );
  }
  const systemProofInputs = createSystemProofInputs({
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
    eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
    relayerRecipientSol: relayer.accounts.relayerRecipientSol,
    signer: transaction.public.accounts.relayerPublicKey,
  });

  const instructions = await createSolanaInstructions({
    action: transaction["action"] ? transaction["action"] : Action.TRANSFER,
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
