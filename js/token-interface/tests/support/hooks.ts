import { setDefaultTimeout } from '@cucumber/cucumber';

// E2E tests interact with on-chain state and need long timeouts (350 seconds).
// Unit tests complete in <100ms naturally.
setDefaultTimeout(350_000);
