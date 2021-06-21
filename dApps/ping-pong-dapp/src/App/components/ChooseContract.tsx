import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import * as React from "react";
import { config } from "../../config/network";
import { getErrorFromStackTrace } from "../../utils/errors";
import { PingPongContract } from "../../utils/PingPong";

interface ChooseContractProps {
  readonly userAddress: string;
  readonly signingClient: SigningCosmWasmClient;
  readonly setPingPongInstance: React.Dispatch<
    React.SetStateAction<PingPongContract | undefined>
  >;
  readonly setError: React.Dispatch<React.SetStateAction<string>>;
}

export default function ChooseContract({
  userAddress,
  signingClient,
  setPingPongInstance,
  setError,
}: ChooseContractProps): JSX.Element {
  const [contractAddress, setContractAddress] = React.useState("");

  const instantiateContract = React.useCallback(async () => {
    try {
      const contractAddress = await PingPongContract.instantiate(
        userAddress,
        signingClient,
        config.pingPongCodeId
      );
      setContractAddress(contractAddress);

      const pingPongInstance = new PingPongContract(
        contractAddress,
        signingClient
      );
      setPingPongInstance(pingPongInstance);
    } catch (error) {
      setError(getErrorFromStackTrace(error));
    }
  }, [setError, setPingPongInstance, signingClient, userAddress]);

  return (
    <div>
      <div className="w-96 rounded-lg overflow-hidden shadow-lg bg-gray-700 border border-purple-700 m-4 flex flex-col">
        <div className="p-6 flex-grow">
          <div className="text-white font-medium text-xl mb-2">
            Enter a contract address
          </div>
          <input
            className="shadow appearance-none border rounded py-2 px-3 w-full text-grey-darker"
            value={contractAddress}
            onChange={({ target: { value } }) => setContractAddress(value)}
          />
          <button
            className="bg-indigo-500 text-white font-medium text-sm mt-3 py-1 px-5 rounded"
            onClick={() =>
              setPingPongInstance(
                new PingPongContract(contractAddress, signingClient)
              )
            }
          >
            Use contract address
          </button>
          <button
            className="bg-red-500 text-white font-medium text-sm mt-3 py-1 px-5 rounded mr-3"
            onClick={() => instantiateContract()}
          >
            Or instantiate new contract
          </button>
        </div>
      </div>
    </div>
  );
}
