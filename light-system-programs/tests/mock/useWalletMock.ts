import {
  Connection,
  Keypair,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { PublicKey } from "@solana/web3.js";

// Mock Solana web3 library
class MockProvider {
  _publicKey: PublicKey;
  _keypair: Keypair;
  _connection: Connection;

  constructor(keypair: Keypair, connection: Connection) {
    this._publicKey = keypair.publicKey;
    this._keypair = keypair;
    this._connection = connection;
  }

  signTransaction = async (tx) => {
    await tx.sign([this._keypair!]);
    return tx;
  };

  signMessage = async (message) => {
    return sign.detached(message, this._keypair.secretKey);
  };

  sendAndConfirmTransaction = async (transaction, signers = []) => {
    try {

      const response =await sendAndConfirmTransaction(this._connection, transaction, [this._keypair, ...signers],{
        commitment:"confirmed"
      })
      console.log(response)
      return response
    // console.log({connection: this._connection})
    // let blockhash = (await this._connection.getLatestBlockhash("finalized"))
    //     .blockhash;
    //   transaction.recentBlockhash = blockhash;
    //   const signedTransaction = transaction.partialSign(this._keypair);

    //   console.log({ signedTransaction });

    //   const transactionId = await this._connection.sendRawTransaction(
    //     signedTransaction.serialize(),
    //   );

    //   console.log({ transactionId });

    //   // Wait for confirmation
    //   const confirmationStatus = await this._connection.confirmTransaction(
    //     transactionId,
    //   );

    //   console.log({ confirmationStatus });

    //   return confirmationStatus;
    } catch (error) {
      console.log("errrr", error);
      throw error;
    }
  };
}

// Mock useWallet hook
export const useWallet = (wallet: Keypair, connection: Connection) => {
  console.log(connection)
  const provider = new MockProvider(wallet, connection);
  return {
    publicKey: provider._publicKey,
    sendAndConfirmTransaction: provider.sendAndConfirmTransaction,
    signMessage: provider.signMessage,
    signTransaction: provider.signTransaction,
  };
};
