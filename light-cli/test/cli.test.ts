import { execSync } from "child_process";

function confirmBalance(logCheck: string[], flag?: string) {
  process.env.BALANCE = JSON.stringify(logCheck);
  if (flag) process.env.FLAG = flag;
  execSync("yarn test-balance", { stdio: 'inherit' })
}

try {
  execSync("npx light test-validator", { stdio: 'inherit' });
  confirmBalance(['SOL   0       0']);
  execSync("yarn test-airdrop", { stdio: 'inherit' });
  execSync("yarn test-shield1", { stdio: 'inherit' });
  confirmBalance(['SOL   7       1', 'USDC  9       1']);
  execSync("yarn test-unshield1", { stdio: 'inherit' });
  confirmBalance(['SOL   0       0', 'USDC  8.5     1']);
  execSync("yarn test-shield:sol", { stdio: 'inherit' });
  confirmBalance(['SOL   2.423456789 1', 'USDC  8.5         1']);
  execSync("yarn test-transfer", { stdio: 'inherit' });
  confirmBalance([
    'SOL   0.922456789 1',
    'USDC  3.5         1',
    'SOL   1.5     1',
    'USDC  5       1',
  ],'-i');
  execSync("yarn test-accept_utxos", { stdio: 'inherit' });
  confirmBalance([
    'SOL   2.421456789 1',
    'USDC  8.5         1',
    'SOL   0       0',
    'USDC  0       0',
  ],'-i');


  } catch (err) {
    console.error(`${err}`);
  }
