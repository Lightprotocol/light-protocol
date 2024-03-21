import { Connection, Signer } from '@solana/web3.js';
import { confirmTx, getTestKeypair } from '@lightprotocol/stateless.js';

export async function newAccountWithLamports(
    connection: Connection,
    lamports = 1000000000,
    counter: number | undefined = undefined,
): Promise<Signer> {
    const account = getTestKeypair(counter);
    const sig = await connection.requestAirdrop(account.publicKey, lamports);
    await confirmTx(connection, sig);
    return account;
}

export function getConnection(): Connection {
    const url = 'http://127.0.0.1:8899';
    const connection = new Connection(url, 'confirmed');
    return connection;
}
