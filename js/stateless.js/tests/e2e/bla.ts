import {
    Rpc,
    createRpc,
} from "@lightprotocol/stateless.js";
import { transfer } from "@lightprotocol/compressed-token";

// Assumes mint, payer and recipient accounts are already created
const connection: Rpc = createRpc();
const transferTxId = await transfer(
    connection,
    payer,
    mint,
    1e9,
    payer,
    tokenRecipient.publicKey
);
console.log("Transaction Signature:", transferTxId);
