import type { MetricsResponse } from "@/types/forester";
import { formatNumber, formatSol } from "@/lib/utils";

interface MetricsPanelProps {
  metrics: MetricsResponse;
}

export function MetricsPanel({ metrics }: MetricsPanelProps) {
  const totalTx = Object.values(metrics.transactions_processed_total).reduce(
    (a, b) => a + b,
    0
  );
  const rates = Object.entries(metrics.transaction_rate);
  const balances = Object.entries(metrics.forester_balances);
  const queues = Object.entries(metrics.queue_lengths);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <StatCard label="Total Transactions" value={formatNumber(totalTx)} />
        <StatCard
          label="Last Run"
          value={
            metrics.last_run_timestamp > 0
              ? new Date(metrics.last_run_timestamp * 1000).toLocaleString()
              : "N/A"
          }
        />
        <StatCard
          label="Active Epochs"
          value={String(
            Object.keys(metrics.transactions_processed_total).length
          )}
        />
      </div>

      {rates.length > 0 && (
        <Section title="Transaction Rate by Epoch">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            {rates.map(([epoch, rate]) => (
              <div key={epoch} className="bg-gray-50 rounded p-3">
                <div className="text-xs text-gray-500">Epoch {epoch}</div>
                <div className="text-sm font-mono font-medium">
                  {rate.toFixed(2)} tx/s
                </div>
                <div className="text-xs text-gray-400 font-mono">
                  {formatNumber(
                    metrics.transactions_processed_total[epoch] ?? 0
                  )}{" "}
                  total
                </div>
              </div>
            ))}
          </div>
        </Section>
      )}

      {balances.length > 0 && (
        <Section title="Forester Balances">
          <div className="space-y-2">
            {balances.map(([pubkey, balance]) => (
              <div
                key={pubkey}
                className="flex justify-between items-center text-xs"
              >
                <span className="font-mono text-gray-700" title={pubkey}>
                  {pubkey.slice(0, 8)}...{pubkey.slice(-4)}
                </span>
                <span
                  className={`font-mono ${balance < 0.1 ? "text-red-600 font-medium" : "text-gray-600"}`}
                >
                  {formatSol(balance)}
                </span>
              </div>
            ))}
          </div>
        </Section>
      )}

      {queues.length > 0 && (
        <Section title="Queue Lengths">
          <div className="space-y-2">
            {queues.map(([tree, length]) => (
              <div
                key={tree}
                className="flex justify-between items-center text-xs"
              >
                <span className="font-mono text-gray-700" title={tree}>
                  {tree.slice(0, 8)}...{tree.slice(-4)}
                </span>
                <span className="font-mono text-gray-600">
                  {formatNumber(length)}
                </span>
              </div>
            ))}
          </div>
        </Section>
      )}
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <div className="text-xs text-gray-500">{label}</div>
      <div className="text-xl font-semibold text-gray-900 mt-1">{value}</div>
    </div>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4">
      <h3 className="text-sm font-semibold text-gray-900 mb-3">{title}</h3>
      {children}
    </div>
  );
}
