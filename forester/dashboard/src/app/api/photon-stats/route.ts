import { NextResponse } from "next/server";
import { Pool } from "pg";

export const dynamic = "force-dynamic";

let pool: Pool | null = null;

function getPool(): Pool | null {
  const url = process.env.PHOTON_DATABASE_URL;
  if (!url) return null;
  if (!pool) {
    pool = new Pool({ connectionString: url, max: 2, idleTimeoutMillis: 30_000 });
  }
  return pool;
}

export async function GET() {
  const db = getPool();
  if (!db) {
    return NextResponse.json(
      { error: "PHOTON_DATABASE_URL not configured" },
      { status: 503 }
    );
  }

  try {
    const client = await db.connect();
    try {
      const [accounts, tokens, compressed] = await Promise.all([
        client.query(`
          SELECT
            COUNT(*) AS total,
            COUNT(*) FILTER (WHERE NOT spent) AS active
          FROM accounts
        `),
        client.query(`
          SELECT
            COUNT(*) AS total,
            COUNT(*) FILTER (WHERE NOT spent) AS active
          FROM token_accounts
        `),
        client.query(`
          SELECT
            encode(a.owner, 'base64') AS owner_b64,
            COUNT(*) AS total,
            COUNT(*) FILTER (WHERE NOT a.spent) AS active
          FROM accounts a
          WHERE a.onchain_pubkey IS NOT NULL
          GROUP BY a.owner
          ORDER BY COUNT(*) DESC
        `),
      ]);

      const totalAccounts = Number(accounts.rows[0].total);
      const activeAccounts = Number(accounts.rows[0].active);
      const totalTokens = Number(tokens.rows[0].total);
      const activeTokens = Number(tokens.rows[0].active);

      const compressedFromOnchain = compressed.rows.map((r: { owner_b64: string; total: string; active: string }) => ({
        owner: Buffer.from(r.owner_b64, "base64").toString("hex"),
        total: Number(r.total),
        active: Number(r.active),
      }));

      const totalCompressedFromOnchain = compressedFromOnchain.reduce(
        (sum: number, r: { total: number }) => sum + r.total,
        0
      );
      const activeCompressedFromOnchain = compressedFromOnchain.reduce(
        (sum: number, r: { active: number }) => sum + r.active,
        0
      );

      return NextResponse.json({
        accounts: { total: totalAccounts, active: activeAccounts },
        token_accounts: { total: totalTokens, active: activeTokens },
        compressed_from_onchain: {
          total: totalCompressedFromOnchain,
          active: activeCompressedFromOnchain,
          by_owner: compressedFromOnchain,
        },
        timestamp: Math.floor(Date.now() / 1000),
      });
    } finally {
      client.release();
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
