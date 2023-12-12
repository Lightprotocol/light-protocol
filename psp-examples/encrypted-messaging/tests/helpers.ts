import { utils } from "@coral-xyz/anchor";
import {
  ConfirmOptions,
  CONSTANT_SECRET_AUTHKEY,
  SendVersionedTransactionsResult,
  User,
} from "@lightprotocol/zk.js";
import nacl from "tweetnacl";
export const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
export class MessageClient {
  constructor(public user: User) {}

  async encryptAndStoreForRecipient(
    message: string,
    recipient: Uint8Array,
  ): Promise<SendVersionedTransactionsResult> {
    const buf = this.str2buf(message);
    const nonce = newNonce();
    const ciphertext = nacl.box(buf, nonce, recipient, CONSTANT_SECRET_AUTHKEY);

    const res = Uint8Array.from([...nonce, ...ciphertext]);
    return this.store(Buffer.from(res));
  }

  async storeString(message: string): Promise<SendVersionedTransactionsResult> {
    const buf = this.str2buf(message);
    return this.store(buf);
  }

  async store(
    message: Buffer,
    anonymousSender: boolean = false,
  ): Promise<SendVersionedTransactionsResult> {
    let res = await this.user.storeData(
      message,
      ConfirmOptions.spendable,
      !anonymousSender,
    );
    console.log("store program utxo transaction hash ", res.txHash);
    return res.txHash;
  }

  async getMessages() {
    let transactions = await this.user.provider.relayer.getIndexedTransactions(
      this.user.provider.connection,
    );
    for (let tx of transactions) {
      if (tx.message != undefined) {
        let decryptedMessage = this.decryptMessage(tx.message);
        if (decryptedMessage == null) {
          decryptedMessage = utils.bytes.utf8.decode(tx.message);
        }
        console.log(decryptedMessage);
      }
    }
  }

  decryptMessage(message: Buffer): string | null {
    const cleartext = nacl.box.open(
      Uint8Array.from(message).slice(nacl.box.nonceLength),
      Uint8Array.from(message).slice(0, nacl.box.nonceLength),
      nacl.box.keyPair.fromSecretKey(CONSTANT_SECRET_AUTHKEY).publicKey,
      this.user.account.encryptionKeypair.secretKey,
    );
    if (cleartext == null) {
      return null;
    }
    return utils.bytes.utf8.decode(Buffer.from(cleartext));
  }

  private str2buf(message: string) {
    return Buffer.from(utils.bytes.utf8.encode(message));
  }
}
