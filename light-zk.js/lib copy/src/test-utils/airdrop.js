"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.airdropSplToAssociatedTokenAccount = exports.airdropShieldedMINTSpl = exports.airdropSol = exports.airdropShieldedSol = void 0;
const anchor_1 = require("@coral-xyz/anchor");
const spl_token_1 = require("@solana/spl-token");
const web3_js_1 = require("@solana/web3.js");
const index_1 = require("../index");
async function airdropShieldedSol({ provider, amount, seed, recipientPublicKey, }) {
    if (!amount)
        throw new Error("Sol Airdrop amount undefined");
    if (!seed && !recipientPublicKey)
        throw new Error("Sol Airdrop seed and recipientPublicKey undefined define a seed to airdrop shielded sol aes encrypted, define a recipientPublicKey to airdrop shielded sol to the recipient nacl box encrypted");
    const relayer = new index_1.TestRelayer({
        relayerPubkey: index_1.ADMIN_AUTH_KEYPAIR.publicKey,
        relayerRecipientSol: web3_js_1.Keypair.generate().publicKey,
        relayerFee: index_1.RELAYER_FEE,
        payer: index_1.ADMIN_AUTH_KEYPAIR,
    });
    if (!provider) {
        provider = await index_1.Provider.init({
            wallet: index_1.ADMIN_AUTH_KEYPAIR,
            relayer: relayer,
            confirmConfig: index_1.confirmConfig,
        });
    }
    const userKeypair = web3_js_1.Keypair.generate();
    await airdropSol({
        connection: provider.provider.connection,
        recipientPublicKey: userKeypair.publicKey,
        lamports: amount * 1e9,
    });
    const user = await index_1.User.init({ provider, seed });
    return await user.shield({
        publicAmountSol: amount,
        token: "SOL",
        recipient: recipientPublicKey,
    });
}
exports.airdropShieldedSol = airdropShieldedSol;
async function airdropSol({ connection, lamports, recipientPublicKey, }) {
    const txHash = await connection.requestAirdrop(recipientPublicKey, lamports);
    await (0, index_1.confirmTransaction)(connection, txHash);
    return txHash;
}
exports.airdropSol = airdropSol;
/**
 * airdrops shielded spl tokens from ADMIN_AUTH_KEYPAIR to the user specified by seed if aes encrypted desired, or by recipient pubkey if nacl box encrypted (will be in utxoInbox then)
 * @param param0
 * @returns
 */
async function airdropShieldedMINTSpl({ provider, amount, seed, recipientPublicKey, }) {
    if (!amount)
        throw new Error("Sol Airdrop amount undefined");
    if (!seed && !recipientPublicKey)
        throw new Error("Sol Airdrop seed and recipientPublicKey undefined define a seed to airdrop shielded sol aes encrypted, define a recipientPublicKey to airdrop shielded sol to the recipient nacl box encrypted");
    const relayer = new index_1.TestRelayer({
        relayerPubkey: index_1.ADMIN_AUTH_KEYPAIR.publicKey,
        relayerRecipientSol: web3_js_1.Keypair.generate().publicKey,
        relayerFee: index_1.RELAYER_FEE,
        payer: index_1.ADMIN_AUTH_KEYPAIR,
    });
    if (!provider) {
        provider = await index_1.Provider.init({
            wallet: index_1.ADMIN_AUTH_KEYPAIR,
            relayer: relayer,
            confirmConfig: index_1.confirmConfig,
        });
    }
    let tokenAccount = await (0, spl_token_1.getOrCreateAssociatedTokenAccount)(provider.provider.connection, index_1.ADMIN_AUTH_KEYPAIR, index_1.MINT, index_1.ADMIN_AUTH_KEYPAIR.publicKey);
    if (new anchor_1.BN(tokenAccount.amount.toString()).toNumber() < amount) {
        await airdropSplToAssociatedTokenAccount(provider.provider.connection, 1000000000000 ? amount : 1000000000000, index_1.ADMIN_AUTH_KEYPAIR.publicKey);
    }
    const user = await index_1.User.init({ provider, seed });
    return await user.shield({
        publicAmountSpl: amount,
        token: index_1.TOKEN_PUBKEY_SYMBOL.get(index_1.MINT.toBase58()),
        recipient: recipientPublicKey,
        skipDecimalConversions: true,
        confirmOptions: index_1.ConfirmOptions.spendable,
    });
}
exports.airdropShieldedMINTSpl = airdropShieldedMINTSpl;
async function airdropSplToAssociatedTokenAccount(connection, lamports, recipient) {
    let tokenAccount = await (0, spl_token_1.getOrCreateAssociatedTokenAccount)(connection, index_1.ADMIN_AUTH_KEYPAIR, index_1.MINT, recipient);
    return await (0, spl_token_1.mintTo)(connection, index_1.ADMIN_AUTH_KEYPAIR, index_1.MINT, tokenAccount.address, index_1.ADMIN_AUTH_KEYPAIR.publicKey, lamports, []);
}
exports.airdropSplToAssociatedTokenAccount = airdropSplToAssociatedTokenAccount;
//# sourceMappingURL=airdrop.js.map