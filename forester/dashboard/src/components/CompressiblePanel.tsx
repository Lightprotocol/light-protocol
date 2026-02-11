import type { CompressibleResponse } from "@/types/forester";
import { formatNumber } from "@/lib/utils";

interface CompressiblePanelProps {
  data: CompressibleResponse;
}

export function CompressiblePanel({ data }: CompressiblePanelProps) {
  if (!data.enabled) {
    return (
      <div className="bg-white rounded-lg border border-gray-200 p-8 text-center">
        <p className="text-gray-500 text-sm">
          No compressible account data available.
        </p>
        <p className="text-gray-400 text-xs mt-2">
          The dashboard will query on-chain data automatically. If this
          persists, check the RPC connection.
        </p>
      </div>
    );
  }

  const cards = [
    {
      label: "CToken Accounts",
      value: data.ctoken_count,
      desc: "Compressed token accounts tracked",
    },
    {
      label: "PDA Accounts",
      value: data.pda_count,
      desc: "Program-derived accounts tracked",
    },
    {
      label: "Mint Accounts",
      value: data.mint_count,
      desc: "Mint accounts tracked",
    },
  ];

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {cards.map((card) => (
          <div
            key={card.label}
            className="bg-white rounded-lg border border-gray-200 p-4"
          >
            <div className="text-xs text-gray-500">{card.label}</div>
            <div className="text-2xl font-semibold text-gray-900 mt-1">
              {card.value != null ? formatNumber(card.value) : "-"}
            </div>
            <div className="text-xs text-gray-400 mt-1">{card.desc}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
