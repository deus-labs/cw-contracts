import * as React from "react";
import cosmWasmLogo from "../assets/cosmWasmLogo.svg";

export default function AppBrand(): JSX.Element {
  return (
    <div>
      <div className="w-96 rounded-lg overflow-hidden shadow-lg bg-gray-700 border border-purple-700 m-4 flex flex-col">
        <div className="pt-6 flex justify-center">
          <img alt="CosmWasm logo" src={cosmWasmLogo} className="w-1/5" />
        </div>
        <div className="px-6 py-6 flex-grow">
          <h1 className="text-center text-white font-medium text-xl mb-2">
            Ping Pong dApp
          </h1>
          <p className="break-words text-white text-l">
            Showcase CosmJs interacting with a CosmWasm contract
          </p>
        </div>
      </div>
    </div>
  );
}
