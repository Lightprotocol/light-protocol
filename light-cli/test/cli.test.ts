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
  execSync("yarn test-shield", { stdio: 'inherit' });
  confirmBalance(['SOL   7       1', 'USDC  10      1']);
  execSync("yarn test-unshield:spl", { stdio: 'inherit' });
  execSync("yarn test-unshield:sol", { stdio: 'inherit' });
  confirmBalance(['SOL   6.7999  1', 'USDC  9.5     1']);
  execSync("yarn test-shield:sol", { stdio: 'inherit' });
  confirmBalance(['SOL   9.223356789 1', 'USDC  9.5         1']);
  execSync("yarn test-transfer", { stdio: 'inherit' });
  confirmBalance([
    'SOL   7.723256789 1',
    'USDC  4.5         1',
    'SOL   1.5     1',
    'USDC  5       1',
  ],'-i');
  execSync("yarn test-accept_utxos", { stdio: 'inherit' });
  confirmBalance([
    'SOL   9.223156789 1',
    'USDC  9.5         1',
    'SOL   0       0',
    'USDC  0       0',
  ],'-i');


  } catch (error) {
    console.error(`${error}`);
  }
