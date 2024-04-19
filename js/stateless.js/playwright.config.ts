import { PlaywrightTestConfig } from '@playwright/test';

const config: PlaywrightTestConfig = {
    webServer: {
        command: 'npx http-server -p 4004 -c-1',
        port: 4004,
        cwd: '.',
        timeout: 15 * 1000,
    },
    testDir: './tests/e2e/browser',
};

export default config;
