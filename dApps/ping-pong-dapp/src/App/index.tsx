import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import * as React from "react";
import { config } from "../config/network";
import { createSigningClient } from "../utils/client";
import { getErrorFromStackTrace } from "../utils/errors";
import { PingPongContract } from "../utils/PingPong";
import { loadOrCreateWallet } from "../utils/wallet";
import AppBrand from "./components/AppBrand";
import ChooseContract from "./components/ChooseContract";
import ErrorBanner from "./components/ErrorBanner";
import UseContract from "./components/UseContract";
import UserAccount from "./components/UserAccount";

export default function App(): JSX.Element {
  const [userAddress, setUserAddress] = React.useState("");
  const [signingClient, setSigningClient] =
    React.useState<SigningCosmWasmClient>();
  const [pingPongInstance, setPingPongInstance] =
    React.useState<PingPongContract>();
  const [error, setError] = React.useState("");

  React.useEffect(() => {
    (async function updateAddressAndClient() {
      try {
        const wallet = await loadOrCreateWallet(config);

        const userAddress = (await wallet.getAccounts())[0].address;
        setUserAddress(userAddress);

        const signingClient = await createSigningClient(config, wallet);
        setSigningClient(signingClient);
      } catch (error) {
        setError(getErrorFromStackTrace(error));
      }
    })();
  }, []);

  return (
    <main className="min-h-screen bg-gray-600 grid grid-cols-1 md:grid-cols-2 content-center justify-items-center">
      <AppBrand />
      {userAddress && signingClient && (
        <ChooseContract
          userAddress={userAddress}
          signingClient={signingClient}
          setPingPongInstance={setPingPongInstance}
          setError={setError}
        />
      )}
      {userAddress && signingClient && (
        <UserAccount
          userAddress={userAddress}
          signingClient={signingClient}
          setError={setError}
        />
      )}
      {userAddress && signingClient && pingPongInstance && (
        <UseContract
          userAddress={userAddress}
          pingPongInstance={pingPongInstance}
          setError={setError}
        />
      )}
      {error ? (
        <ErrorBanner error={error} clearError={() => setError("")} />
      ) : null}
    </main>
  );
}
