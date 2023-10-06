import "dotenv/config.js";
import { Queue, Worker } from "bullmq";
import IORedis from "ioredis";
import {
  ATTEMPTS,
  CONCURRENT_RELAY_WORKERS,
  Environment,
  HOST,
  PASSWORD,
  PORT,
} from "../config";

import { sendVersionedTransactions } from "@lightprotocol/zk.js";
import { getLightProvider } from "../utils/provider";
import { parseReqParams } from "../services/index";

let redisConnection: any;

if (process.env.ENVIRONMENT === Environment.PROD) {
  redisConnection = new IORedis(Number(PORT), HOST, {
    username: "default",
    password: PASSWORD,
    tls: {},
    maxRetriesPerRequest: 20,
    connectTimeout: 20_000,
  });
  console.log("using REMOTE REDIS");
} else if (process.env.ENVIRONMENT === Environment.LOCAL) {
  console.log(process.env.ENVIRONMENT);
  redisConnection = new IORedis({ maxRetriesPerRequest: null });
} else {
  throw new Error("Please provide ENVIRONMENT env varibale (LOCAL/PROD)!");
}

export const getDbConnection = async () => {
  if (!redisConnection) throw new Error("REDIS env not configured correctly!");
  return redisConnection;
};

export const relayQueue = new Queue("relay", {
  connection: redisConnection,
  defaultJobOptions: {
    attempts: ATTEMPTS,
    backoff: {
      type: "exponential",
      delay: 2000,
    },
  },
});
// TODO: extract into a separate db system for optimizing performance at scale
export const indexQueue = new Queue("index", {
  connection: redisConnection,
});

console.log("Queues activated");

export const relayWorker = new Worker(
  "relay",
  async (job) => {
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
        // fetch newes job data and print
        throw new Error(response.error.message);
      }
      await job.updateData({
        ...job.data,
        response,
      });
      // this is not yet returned
    } catch (e) {
      console.log(e);
      throw e;
    }
    return true;
  },
  { connection: redisConnection, concurrency: CONCURRENT_RELAY_WORKERS },
);

relayWorker.on("completed", async (job) => {
  const duration = Date.now() - job.timestamp;
  const message = `relay: ${job.id} completed! duration: ${duration / 1000}s`;
  console.log(message);
});

relayWorker.on("failed", async (job, err) => {
  if (job) {
    if (job.attemptsMade < ATTEMPTS) {
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
