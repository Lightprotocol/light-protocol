import "dotenv/config.js";
import { Queue, Worker, Job } from "bullmq";
import IORedis from "ioredis";
import {
  WORKER_RETRIES_PER_JOB,
  CONCURRENT_RELAY_WORKERS,
  Environment,
  HOST,
  PASSWORD,
  PORT,
  REDIS_ENVIRONMENT,
} from "../config";

import { sendVersionedTransactions } from "@lightprotocol/zk.js";
import { getLightProvider } from "../utils/provider";
import { parseReqParams } from "../services/index";
import {
  EnvironmentVariableError,
  EnvironmentVariableErrorCode,
  RedisError,
  RedisErrorCode,
} from "../errors";

let redisConnection: any;

if (REDIS_ENVIRONMENT === Environment.PROD) {
  redisConnection = new IORedis(Number(PORT), HOST, {
    username: "default",
    password: PASSWORD,
    tls: {},
    maxRetriesPerRequest: 20,
    connectTimeout: 20_000,
  });
  console.log("using REMOTE REDIS");
} else if (REDIS_ENVIRONMENT === Environment.LOCAL) {
  console.log(REDIS_ENVIRONMENT);
  redisConnection = new IORedis({ maxRetriesPerRequest: null });
} else {
  throw new EnvironmentVariableError(
    EnvironmentVariableErrorCode.VARIABLE_NOT_SET,
    "redis.ts",
    "provide either LOCAL or PROD for REDIS_ENVIRONMENT",
  );
}

export const getDbConnection = async () => {
  if (!redisConnection)
    throw new RedisError(RedisErrorCode.NO_REDIS_CONNECTION, "redis.ts");
  return redisConnection;
};

export const relayQueue = new Queue("relay", {
  connection: redisConnection,
  defaultJobOptions: {
    attempts: WORKER_RETRIES_PER_JOB,
  },
});
// TODO: move to a separate db system for optimizing performance at scale
export const indexQueue = new Queue("index", {
  connection: redisConnection,
});

console.log("Queues activated");

export const relayWorker = new Worker(
  "relay",
  async (job: Job) => {
    console.log(`/relayWorker relay start - id: ${job.id}`);
    const { instructions } = job.data;
    const parsedInstructions = await parseReqParams(instructions);
    try {
      const provider = await getLightProvider();
      const response = await sendVersionedTransactions(
        parsedInstructions,
        provider.provider!.connection,
        provider.lookUpTables.versionedTransactionLookupTable!,
        provider.wallet,
      );
      console.log("RELAY  JOB WORKER SENT TX, RESPONSE: ", response);
      if (response.error) {
        await job.updateData({
          ...job.data,
          response: { error: response.error.message },
        });
        throw new Error(response.error.message);
      }
      await job.updateData({
        ...job.data,
        response,
      });
    } catch (e) {
      console.log("error in worker: ", e);
      throw e;
    }
  },
  { connection: redisConnection, concurrency: CONCURRENT_RELAY_WORKERS },
);

relayQueue.on("error", (err) => {
  console.log("relayQueue error:", err);
});
relayQueue.on("waiting", async (job: Job) => {
  console.log(`relay: ${job.id} waiting!`);
});

process.on("uncaughtException", function (err) {
  console.error(err, "Uncaught exception");
});
process.on("unhandledRejection", (reason, promise) => {
  console.error({ promise, reason }, "Unhandled Rejection at: Promise");
});

relayWorker.on("active", async (job: Job) => {
  console.log(`relay: ${job.id} active!`);
});

relayWorker.on("completed", async (job: Job) => {
  const duration = Date.now() - job.timestamp;
  const message = `relay: ${job.id} completed! duration: ${duration / 1000}s`;
  console.log(message);
});

relayWorker.on("failed", async (job, err) => {
  console.log("relayWorker failed", err);
  if (job) {
    if (job.attemptsMade < WORKER_RETRIES_PER_JOB) {
      console.log(
        `relay #${job.id} attempt ${job.attemptsMade} failed - retrying`,
      );
      return;
    }
    const duration = Date.now() - job!.timestamp;
    const message = `relay ${job.id} failed (${err.message}) after ${
      duration / 1000
    }s`;
    console.log(message);
    console.log(
      `relay (job: ${job.id} failed after ${job.attemptsMade} attempts - exiting`,
    );
    await job.updateData({ ...job.data, response: { error: message } });

    return message;
  }
});

export const getTransactions = async (version = 0) => {
  const job = (await indexQueue.getWaiting())[version];
  console.log("getTransactions", version, job ? "job" : "no job");
  if (job) {
    return { transactions: job.data.transactions, job };
  } else {
    const newJob = await indexQueue.add("indexJob", {
      transactions: [],
      lastFetched: 0,
    });
    console.log("Initialized RecentTx job");
    return { transactions: [], job: newJob };
  }
};
