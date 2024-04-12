import { describe, it, assert, beforeAll, expect } from 'vitest';
import { Signer } from '@solana/web3.js';
import { defaultTestStateTreeAccounts } from '../../src/constants';
import { getTestRpc, newAccountWithLamports } from '../../src/test-utils';
import {
    CompressedAccountWithMerkleContext,
    Rpc,
    bn,
    compressLamports,
    createRpc,
    initSolOmnibusAccount,
} from '../../src';

/// TODO: add test case for payer != address
describe('rpc / photon', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: Rpc;
    let payer: Signer;
    let initAuthority: Signer;
    let preCompressBalance: number;
    let postCompressBalance: number;
    let compressLamportsAmount: number;
    let compressedTestAccount: CompressedAccountWithMerkleContext;
    /// 0th leaf
    const refHash: number[] = [
        27, 94, 128, 101, 38, 3, 38, 161, 60, 238, 2, 229, 53, 162, 108, 59,
        239, 144, 75, 88, 68, 221, 112, 179, 146, 27, 92, 4, 195, 153, 23, 48,
    ];
    /// 0th leaf merkle proof
    const refMerkleProof: string[] = [
        '0',
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
        rpc = createRpc();

        payer = await newAccountWithLamports(rpc, 1e9, 200);
        initAuthority = await newAccountWithLamports(rpc, 1e9);

        /// compress
        compressLamportsAmount = 20;
        preCompressBalance = await rpc.getBalance(payer.publicKey);
        await initSolOmnibusAccount(rpc, initAuthority, initAuthority);
        await compressLamports(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            merkleTree,
        );
    });

    /// always run this test first
    it('getCompressedAccountsByOwner', async () => {
        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        if (!compressedAccounts)
            throw new Error('No compressed accounts found');

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
            preCompressBalance - compressLamportsAmount - 5000,
        );
    });

    it('getCompressedAccountProof', async () => {
        const compressedAccountProof = await rpc.getCompressedAccountProof(
            bn(refHash),
        );

        const proof = compressedAccountProof.merkleProof.map(x => x.toString());

        /// TODO: photon: don't return the root
        expect(proof.slice(0, -1)).toStrictEqual(refMerkleProof);

        await compressLamports(
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

        /// TODO: remove once merkleproof debugged
        const hash2 = compressedAccounts![1].hash;
        await (await getTestRpc()).getValidityProof([bn(hash2)]);
    });

    it('getHealth', async () => {
        /// getHealth
        const health = await rpc.getHealth();
        assert.strictEqual(health, 'ok');
    });

    it('getSlot', async () => {
        /// getSlot
        const slot = await rpc.getSlot();
        assert(slot > 0);
    });

    it('getCompressedAccount', async () => {
        /// getCompressedAccount
        const compressedAccount = await rpc.getCompressedAccount(bn(refHash));
        assert(compressedAccount !== null);
        assert.equal(
            compressedAccount.owner.toBase58(),
            payer.publicKey.toBase58(),
        );
        assert.equal(compressedAccount.data, null);
    });

    it('getCompressedBalance', async () => {
        /// getCompressedBalance
        const compressedBalance = await rpc.getCompressedBalance(bn(refHash));
        expect(compressedBalance?.eq(bn(compressLamportsAmount))).toBeTruthy();

        return;
    });
});
