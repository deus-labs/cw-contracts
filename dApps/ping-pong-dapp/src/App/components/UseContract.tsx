import * as React from "react";
import { getErrorFromStackTrace } from "../../utils/errors";
import { PingPongContract } from "../../utils/PingPong";

interface UseContractProps {
  readonly userAddress: string;
  readonly pingPongInstance: PingPongContract;
  readonly setError: React.Dispatch<React.SetStateAction<string>>;
}

export default function UseContract({
  userAddress,
  pingPongInstance,
  setError,
}: UseContractProps): JSX.Element {
  const [pingCount, setPingCount] = React.useState(0);
  const [pingResult, setPingResult] = React.useState("");

  const queryPingCount = React.useCallback(async () => {
    try {
      const pingCount = await pingPongInstance.queryPingCount();
      setPingCount(pingCount);
    } catch (error) {
      setError(getErrorFromStackTrace(error));
    }
  }, [pingPongInstance, setError]);

  const executePing = React.useCallback(async () => {
    setPingResult("");

    try {
      const transactionHash = await pingPongInstance.executePing(userAddress);
      setPingResult(transactionHash);
    } catch (error) {
      setError(getErrorFromStackTrace(error));
    }
  }, [pingPongInstance, setError, userAddress]);

  React.useEffect(() => {
    queryPingCount();
  }, [queryPingCount]);

  return (
    <div>
      <div className="w-96 rounded-lg overflow-hidden shadow-lg bg-gray-700 border border-purple-700 m-4 flex flex-col">
        <div className="p-6 flex-grow">
          <div className="text-white font-medium text-xl mb-2">Ping count</div>
          <p className="break-words text-white text-sm">{pingCount}</p>
          <button
            className="bg-indigo-500 text-white font-medium text-sm mt-3 py-1 px-5 rounded"
            onClick={() => queryPingCount()}
          >
            Refresh ping count
          </button>
        </div>
        <div className="p-6 border-t border-purple-700 flex-grow">
          <div className="text-white font-medium text-xl mb-2">
            Send Ping transaction
          </div>
          <button
            className="bg-red-500 text-white font-medium text-sm mt-3 py-1 px-5 rounded mr-3"
            onClick={() => executePing()}
          >
            Execute
          </button>
        </div>
        {pingResult ? (
          <div className="p-6 border-t border-purple-700 flex-grow">
            <div className="text-white font-medium text-xl mb-2">
              Received pong with hash:
            </div>
            <p className="break-words text-white text-sm">{pingResult}</p>
          </div>
        ) : null}
      </div>
    </div>
  );
}
