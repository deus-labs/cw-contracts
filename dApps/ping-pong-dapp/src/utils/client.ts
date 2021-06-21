import {
  defaultGasLimits,
  SigningCosmWasmClient,
} from "@cosmjs/cosmwasm-stargate";
import { OfflineDirectSigner } from "@cosmjs/proto-signing";
import { GasPrice } from "@cosmjs/stargate";
import { NetworkConfig } from "../config/network";

export async function createSigningClient(
  config: NetworkConfig,
  signer: OfflineDirectSigner
): Promise<SigningCosmWasmClient> {
  const options = {
    prefix: config.addressPrefix,
    gasPrice: GasPrice.fromString(`0.025${config.feeToken}`),
    gasLimits: defaultGasLimits,
  };
  const signingClient = SigningCosmWasmClient.connectWithSigner(
    config.rpcUrl,
    signer,
    options
  );
  return signingClient;
}
