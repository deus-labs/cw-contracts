import * as React from "react";

interface ErrorBannerProps {
  readonly error: string;
  readonly clearError: () => void;
}

export default function ErrorBanner({
  error,
  clearError,
}: ErrorBannerProps): JSX.Element {
  return (
    <footer
      className="z-1 fixed bottom-0 left-0 right-0 cursor-pointer bg-red-200 p-5 items-center"
      onClick={() => clearError()}
    >
      <div className="font-semibold text-lg text-red-800">Error</div>
      <div className="text-sm text-red-600">{error}</div>
    </footer>
  );
}
