interface ErrorStateProps {
  error: Error | undefined;
  fallbackMessage?: string;
}

export function ErrorState({ error, fallbackMessage }: ErrorStateProps) {
  if (!error) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-center max-w-lg">
          <p className="text-red-500 text-sm font-medium">Error</p>
          <p className="text-gray-400 text-xs mt-1">
            {fallbackMessage || "An unexpected error occurred"}
          </p>
        </div>
      </div>
    );
  }

  // ApiError sets name="ApiError" and message starts with "Forester API returned"
  const isApiError =
    error.name === "ApiError" ||
    error.message.startsWith("Forester API returned");

  return (
    <div className="flex items-center justify-center h-64">
      <div className="text-center max-w-lg">
        {isApiError ? (
          <>
            <p className="text-amber-600 text-sm font-medium">API Error</p>
            <p className="text-gray-500 text-xs mt-2">{error.message}</p>
            <p className="text-gray-400 text-xs mt-3">
              The forester API server is reachable but returned an error. Check
              that --rpc-url points to a valid Solana RPC with Light Protocol
              deployed.
            </p>
          </>
        ) : (
          <>
            <p className="text-red-500 text-sm font-medium">
              Connection Error
            </p>
            <p className="text-gray-500 text-xs mt-2">{error.message}</p>
          </>
        )}
      </div>
    </div>
  );
}
