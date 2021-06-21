export interface NetworkConfig {
  readonly chainId: string;
  readonly rpcUrl: string;
  readonly faucetUrl: string;
  readonly addressPrefix: string;
  readonly feeToken: string;
  readonly pingPongCodeId: number;
}
export const config: NetworkConfig = {
  chainId: "oysternet-1",
  addressPrefix: "wasm",
  rpcUrl: "http://rpc.oysternet.cosmwasm.com",
  faucetUrl: "https://faucet.oysternet.cosmwasm.com",
  feeToken: "usponge",
  pingPongCodeId: 15,
};
