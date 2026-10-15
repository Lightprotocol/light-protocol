const fs = require("fs");
const path = require("path");

const EXPECTATIONS = [
  {
    filename:
      "batch_state_merkle_tree_bmt1LryLZUMmF7ZtqESaw7wifBXLfXHQYoE4GAmrahU.json",
    batchSizeOffset: 272,
    zkpBatchSizeOffset: 280,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batch_state_merkle_tree_bmt2UxoBxB9xWev4BkLvkGdapsz6sZGkzViPNph7VFi.json",
    batchSizeOffset: 272,
    zkpBatchSizeOffset: 280,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batch_state_merkle_tree_bmt3ccLd4bqSVZVeCJnH1F6C8jNygAhaDfxDwePyyGb.json",
    batchSizeOffset: 272,
    zkpBatchSizeOffset: 280,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batch_state_merkle_tree_bmt4d3p1a4YQgk9PeZv5s4DBUmbF5NxqYpk9HGjQsd8.json",
    batchSizeOffset: 272,
    zkpBatchSizeOffset: 280,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batch_state_merkle_tree_bmt5yU97jC88YXTuSukYHa8Z5Bi2ZDUtmzfkDTA2mG2.json",
    batchSizeOffset: 272,
    zkpBatchSizeOffset: 280,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batched_output_queue_oq1na8gojfdUhsfCpyjNt6h4JaDWtHf1yQj4koBWfto.json",
    batchSizeOffset: 240,
    zkpBatchSizeOffset: 248,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batched_output_queue_oq2UkeMsJLfXt2QHzim242SUi3nvjJs8Pn7Eac9H9vg.json",
    batchSizeOffset: 240,
    zkpBatchSizeOffset: 248,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batched_output_queue_oq3AxjekBWgo64gpauB6QtuZNesuv19xrhaC1ZM1THQ.json",
    batchSizeOffset: 240,
    zkpBatchSizeOffset: 248,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batched_output_queue_oq4ypwvVGzCUMoiKKHWh4S1SgZJ9vCvKpcz6RT6A8dq.json",
    batchSizeOffset: 240,
    zkpBatchSizeOffset: 248,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batched_output_queue_oq5oh5ZR3yGomuQgFduNDzjtGvVWfDRGLuDVjv9a96P.json",
    batchSizeOffset: 240,
    zkpBatchSizeOffset: 248,
    expectedBatchSize: 15000,
    expectedZkpBatchSize: 500,
  },
  {
    filename:
      "batch_address_merkle_tree_amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx.json",
    batchSizeOffset: 272,
    zkpBatchSizeOffset: 280,
    expectedBatchSize: 30000,
    expectedZkpBatchSize: 250,
  },
];

function readSerializedAccountBuffer(accountsDir, filename) {
  const raw = fs.readFileSync(path.join(accountsDir, filename), "utf8");
  const parsed = JSON.parse(raw);
  return Buffer.from(parsed.account.data[0], "base64");
}

function readU64(buffer, offset) {
  return Number(buffer.readBigUInt64LE(offset));
}

function assertBundledAccountSizes(accountsDir = path.resolve(__dirname, "../accounts")) {
  for (const expectation of EXPECTATIONS) {
    const buffer = readSerializedAccountBuffer(accountsDir, expectation.filename);
    const batchSize = readU64(buffer, expectation.batchSizeOffset);
    const zkpBatchSize = readU64(buffer, expectation.zkpBatchSizeOffset);

    if (batchSize !== expectation.expectedBatchSize) {
      throw new Error(
        `${expectation.filename} has batch_size=${batchSize}, expected ${expectation.expectedBatchSize}`,
      );
    }

    if (zkpBatchSize !== expectation.expectedZkpBatchSize) {
      throw new Error(
        `${expectation.filename} has zkp_batch_size=${zkpBatchSize}, expected ${expectation.expectedZkpBatchSize}`,
      );
    }
  }
}

if (require.main === module) {
  assertBundledAccountSizes();
  console.log("Verified bundled CLI batched account sizes.");
}

module.exports = {
  EXPECTATIONS,
  assertBundledAccountSizes,
};
