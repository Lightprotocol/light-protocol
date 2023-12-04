import {
  Utxo,
  TransactionParameters,
  Action,
  PspTransactionInput,
  MerkleTreeConfig,
  IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
  getVerifierStatePda,
  Account,
  Relayer,
} from "@lightprotocol/zk.js";

import { BN, Program } from "@coral-xyz/anchor";
import { PrivateVoting } from "../target/types/private_voting";
import { PublicKey } from "@solana/web3.js";

export const createPspTransaction = async (
  pspTransactionInput: PspTransactionInput,
  poseidon: any,
  account: Account,
  relayer: Relayer
): Promise<TransactionParameters> => {
  let inputUtxos: Utxo[] = [];
  if (pspTransactionInput.checkedInUtxos) {
    inputUtxos = [
      ...pspTransactionInput.checkedInUtxos.map((item) => item.utxo),
    ];
  }
  if (pspTransactionInput.inUtxos) {
    inputUtxos = [...inputUtxos, ...pspTransactionInput.inUtxos];
  }
  let outputUtxos: Utxo[] = [];
  if (pspTransactionInput.checkedOutUtxos) {
    outputUtxos = [
      ...pspTransactionInput.checkedOutUtxos.map((item) => item.utxo),
    ];
  }
  if (pspTransactionInput.outUtxos) {
    outputUtxos = [...outputUtxos, ...pspTransactionInput.outUtxos];
  }

  const txParams = new TransactionParameters({
    inputUtxos,
    outputUtxos,
    transactionMerkleTreePubkey: MerkleTreeConfig.getTransactionMerkleTreePda(
      new BN(0)
    ),
    eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(new BN(0)),
    action: Action.TRANSFER,
    poseidon,
    relayer: relayer,
    verifierIdl: IDL_LIGHT_PSP4IN4OUT_APP_STORAGE,
    account: account,
    verifierState: getVerifierStatePda(
      TransactionParameters.getVerifierProgramId(
        pspTransactionInput.verifierIdl
      ),
      relayer.accounts.relayerPubkey
    ),
  });

  await txParams.getTxIntegrityHash(poseidon);
  return txParams;
};

export type UnpackedVotePda = {
  publicElGamalPublicKeyX: BN;
  publicElGamalPublicKeyY: BN;
  publicVoteWeightNoEmphemeralKeyX: BN;
  publicVoteWeightNoEmphemeralKeyY: BN;
  publicVoteWeightYesEmphemeralKeyX: BN;
  publicVoteWeightYesEmphemeralKeyY: BN;
  publicVoteWeightNoX: BN;
  publicVoteWeightNoY: BN;
  publicVoteWeightYesX: BN;
  publicVoteWeightYesY: BN;
};

export const fetchAndConvertVotePda = async (
  voteProgram: Program<PrivateVoting>,
  votePda: PublicKey
): Promise<UnpackedVotePda> => {
  const voteAccountInfoPreTx = await voteProgram.account.votePda.fetch(votePda);
  return {
    publicElGamalPublicKeyX: new BN(
      voteAccountInfoPreTx.thresholdEncryptionPubkey[0]
    ),
    publicElGamalPublicKeyY: new BN(
      voteAccountInfoPreTx.thresholdEncryptionPubkey[1]
    ),
    publicVoteWeightNoEmphemeralKeyX: new BN(
      voteAccountInfoPreTx.encryptedNoVotes[0]
    ),
    publicVoteWeightNoEmphemeralKeyY: new BN(
      voteAccountInfoPreTx.encryptedNoVotes[1]
    ),
    publicVoteWeightYesEmphemeralKeyX: new BN(
      voteAccountInfoPreTx.encryptedYesVotes[0]
    ),
    publicVoteWeightYesEmphemeralKeyY: new BN(
      voteAccountInfoPreTx.encryptedYesVotes[1]
    ),
    publicVoteWeightNoX: new BN(voteAccountInfoPreTx.encryptedNoVotes[2]),
    publicVoteWeightNoY: new BN(voteAccountInfoPreTx.encryptedNoVotes[3]),
    publicVoteWeightYesX: new BN(voteAccountInfoPreTx.encryptedYesVotes[2]),
    publicVoteWeightYesY: new BN(voteAccountInfoPreTx.encryptedYesVotes[3]),
  };
};
