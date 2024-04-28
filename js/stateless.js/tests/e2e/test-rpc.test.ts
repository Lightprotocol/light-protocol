import { describe, it, assert, beforeAll, expect } from 'vitest';
import { PublicKey, Signer } from '@solana/web3.js';
import {
    STATE_MERKLE_TREE_ROLLOVER_FEE,
    defaultTestStateTreeAccounts,
} from '../../src/constants';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { compress, decompress } from '../../src/actions';
import {
    bn,
    CompressedAccountWithMerkleContext,
    encodeBN254toBase58,
} from '../../src/state';
import { TestRpc, getTestRpc } from '../../src/test-helpers';
import { createRpc } from '../../src';

/// TODO: add test case for payer != address
describe('test-rpc', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: TestRpc;
    let payer: Signer;

    let preCompressBalance: number;
    let postCompressBalance: number;
    let compressLamportsAmount: number;
    let compressedTestAccount: CompressedAccountWithMerkleContext;
    let refPayer: Signer;
    const refCompressLamports = 1e7;
    /// 0th leaf for refPayer
    /// Note: also depends on testKeypair derivation seed.
    const refHash: number[] = [
        13, 225, 248, 105, 237, 121, 108, 70, 70, 197, 240, 130, 226, 236, 129,
        58, 213, 50, 236, 99, 216, 99, 91, 201, 141, 76, 196, 33, 41, 181, 236,
        187,
    ];
    /// 0th leaf merkle proof for refPayer after 2 compressions
    const refMerkleProof: string[] = [
        '2389670883532368111579825686474970238671018859817536120264461201496859761111',
        '14744269619966411208579211824598458697587494354926760081771325075741142829156',
        '7423237065226347324353380772367382631490014989348495481811164164159255474657',
        '11286972368698509976183087595462810875513684078608517520839298933882497716792',
        '3607627140608796879659380071776844901612302623152076817094415224584923813162',
        '19712377064642672829441595136074946683621277828620209496774504837737984048981',
        '20775607673010627194014556968476266066927294572720319469184847051418138353016',
        '3396914609616007258851405644437304192397291162432396347162513310381425243293',
        '21551820661461729022865262380882070649935529853313286572328683688269863701601',
        '6573136701248752079028194407151022595060682063033565181951145966236778420039',
        '12413880268183407374852357075976609371175688755676981206018884971008854919922',
        '14271763308400718165336499097156975241954733520325982997864342600795471836726',
        '20066985985293572387227381049700832219069292839614107140851619262827735677018',
        '9394776414966240069580838672673694685292165040808226440647796406499139370960',
        '11331146992410411304059858900317123658895005918277453009197229807340014528524',
        '15819538789928229930262697811477882737253464456578333862691129291651619515538',
        '19217088683336594659449020493828377907203207941212636669271704950158751593251',
        '21035245323335827719745544373081896983162834604456827698288649288827293579666',
        '6939770416153240137322503476966641397417391950902474480970945462551409848591',
        '10941962436777715901943463195175331263348098796018438960955633645115732864202',
        '15019797232609675441998260052101280400536945603062888308240081994073687793470',
        '11702828337982203149177882813338547876343922920234831094975924378932809409969',
        '11217067736778784455593535811108456786943573747466706329920902520905755780395',
        '16072238744996205792852194127671441602062027943016727953216607508365787157389',
        '17681057402012993898104192736393849603097507831571622013521167331642182653248',
        '21694045479371014653083846597424257852691458318143380497809004364947786214945',
    ];

    beforeAll(async () => {
        rpc = await getTestRpc();
        refPayer = await newAccountWithLamports(rpc, 1e9, 200);
        payer = await newAccountWithLamports(rpc, 1e9, 148);

        /// compress refPayer
        await compress(
            rpc,
            refPayer,
            refCompressLamports,
            refPayer.publicKey,
            merkleTree,
        );

        /// compress
        compressLamportsAmount = 1e7;
        preCompressBalance = await rpc.getBalance(payer.publicKey);

        await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            merkleTree,
        );
    });

    /// always run this test first per global test suite
    it('rpc should return refAccountHash', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            refPayer.publicKey,
        );
        console.log('compressedAccounts TESTRPC', compressedAccounts);

        const realRpc = createRpc();
        const compressedAccountsReal =
            await realRpc.getCompressedAccountsByOwner(refPayer.publicKey);

        console.log('compressedAccounts REALRPC', compressedAccountsReal);

        assert.equal(compressedAccounts.length, 1);

        expect(bn(compressedAccounts[0].hash).eq(bn(refHash))).toBeTruthy();

        const b58viaBN = encodeBN254toBase58(bn(refHash));
        const b58viaPk = new PublicKey(refHash).toBase58();
        assert.equal(b58viaBN, b58viaPk);

        const arrFromB58 = new PublicKey(b58viaBN).toBytes();

        assert.equal(arrFromB58.length, refHash.length);
        assert.equal(
            arrFromB58.every((v, i) => v === refHash[i]),
            true,
        );

        const refCacct = await rpc.getCompressedAccount(bn(refHash));
        assert.equal(refCacct!.owner.toBase58(), refPayer.publicKey.toBase58());
        assert.equal(bn(refCacct!.hash).eq(bn(refHash)), true);

        const compressedAccount = compressedAccounts[0];
        assert.equal(Number(compressedAccount.lamports), refCompressLamports);
        assert.equal(
            compressedAccount.owner.toBase58(),
            refPayer.publicKey.toBase58(),
        );
        assert.equal(compressedAccount.data?.data, null);
    });

    it('getCompressedAccountsByOwner', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );

        compressedTestAccount = compressedAccounts[0];
        assert.equal(compressedAccounts.length, 1);
        assert.equal(
            Number(compressedTestAccount.lamports),
            compressLamportsAmount,
        );
        assert.equal(
            compressedTestAccount.owner.toBase58(),
            payer.publicKey.toBase58(),
        );
        assert.equal(compressedTestAccount.data?.data, null);

        postCompressBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(
            postCompressBalance,
            preCompressBalance -
                compressLamportsAmount -
                5000 -
                STATE_MERKLE_TREE_ROLLOVER_FEE.toNumber(),
        );
    });

    it('getCompressedAccountProof for refPayer', async () => {
        const compressedAccountProof = await rpc.getCompressedAccountProof(
            bn(refHash),
        );
        const proof = compressedAccountProof.merkleProof.map(x => x.toString());

        expect(proof).toStrictEqual(refMerkleProof);

        expect(compressedAccountProof.hash).toStrictEqual(refHash);
        expect(compressedAccountProof.leafIndex).toStrictEqual(0);
        expect(compressedAccountProof.rootIndex).toStrictEqual(2);

        await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            merkleTree,
        );
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        expect(compressedAccounts?.length).toStrictEqual(2);
    });

    it('getCompressedAccountProof: get many valid proofs (10)', async () => {
        for (let lamports = 1; lamports <= 10; lamports++) {
            await decompress(rpc, payer, lamports, payer.publicKey, merkleTree);
        }
    });
    it('getIndexerHealth', async () => {
        /// getHealth
        const health = await rpc.getIndexerHealth();
        assert.strictEqual(health, 'ok');
    });

    it('getIndexerSlot / getSlot', async () => {
        const slot = await rpc.getIndexerSlot();
        const slotWeb3 = await rpc.getSlot();
        assert(slot > 0);
        assert(slotWeb3 > 0);
    });

    it('getCompressedAccount', async () => {
        /// getCompressedAccount
        const compressedAccount = await rpc.getCompressedAccount(bn(refHash));
        assert(compressedAccount !== null);
        assert.equal(
            compressedAccount.owner.toBase58(),
            refPayer.publicKey.toBase58(),
        );
        assert.equal(compressedAccount.data, null);
    });

    it.skip('getCompressedBalance', async () => {
        /// getCompressedBalance
        const compressedBalance = await rpc.getCompressedBalance(bn(refHash));
        expect(compressedBalance?.eq(bn(refCompressLamports))).toBeTruthy();
    });
});
