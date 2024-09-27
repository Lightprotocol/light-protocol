import { expect, vi } from 'vitest';
import { config } from '@oclif/core';

global.expect = expect;

vi.mock('@oclif/core', async () => {
  const actual = await vi.importActual('@oclif/core');
  return {
    ...actual,
    config: {
      ...actual.config,
      load: vi.fn().mockResolvedValue({}),
    },
  };
});

beforeAll(async () => {
  await config.load();
});