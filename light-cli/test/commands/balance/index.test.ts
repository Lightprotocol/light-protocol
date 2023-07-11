import {expect, test} from '@oclif/test';

function balanceCheck(...logs: string[]) {
    const flag = process.env.FLAG;
    const command = flag ? ['balance', flag] : ['balance'];
    test
    .stdout()
    .command(command)
    .it('deterministic balance checks', (ctx: any) => {
        //console.warn(JSON.stringify(ctx.stdout))
        logs.map(word => expect(ctx.stdout).to.contain(word))
    })  
}

describe('balance', () => {
    //console.log(JSON.parse(process.env.BALANCE!));
    balanceCheck(...JSON.parse(process.env.BALANCE!))
})
