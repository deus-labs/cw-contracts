import { Coin, StdFee } from "@cosmjs/launchpad";
/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the cw20 contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli@^0.26 --init https://raw.githubusercontent.com/CosmWasm/cosmwasm-examples/main/nameservice/helpers.ts --init https://raw.githubusercontent.com/CosmWasm/testnets/master/network.ts
 *
 * Create a client:
 *   const client = await useOptions(coralnetOptions).setup(password);
 *   await client.getAccount()
 *
 * Get the mnemonic:
 *   await useOptions(coralnetOptions).recoverMnemonic(password)
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */


interface Config {
  readonly purchase_price?: Coin
  readonly transfer_price?: Coin
}

interface ResolveRecordResponse {
  readonly address?: string
}

interface InitMsg {

  readonly purchase_price?: Coin
  readonly transfer_price?: Coin
}

interface NameServiceInstance {
  readonly contractAddress: string

  // queries
  record: (name: string) => Promise<ResolveRecordResponse>
  config: () => Promise<Config>

  // actions
  register: (txSigner: string, name: string, amount: Coin[]) => Promise<any>
  transfer: (txSigner: string, name: string, to: string, amount: Coin[]) => Promise<any>
}

interface NameServiceContract {
  upload: () => Promise<number>

  instantiate: (codeId: number, initMsg: InitMsg, label: string) => Promise<NameServiceInstance>

  use: (contractAddress: string) => NameServiceInstance
}

const NameService = (client: SigningCosmWasmClient): NameServiceContract => {
  const use = (contractAddress: string): NameServiceInstance => {
    const record = async (name: string): Promise<ResolveRecordResponse> => {
      return client.queryContractSmart(contractAddress, {resolve_record: { name }});
    };

    const config = async (): Promise<Config> => {
      return client.queryContractSmart(contractAddress, {config: { }});
    };

    const register = async (txSigner: string, name: string, amount: string): Promise<any> => {
      const result = await client.execute(txSigner, contractAddress, {register: { name }}, amount);
      return result.transactionHash;
    };

    const transfer = async (txSigner: string, name: string, to: string, amount: string): Promise<any> => {
      const result = await client.execute(txSigner, contractAddress, {transfer: { name, to }}, );
      return result.transactionHash;
    };

    return {
      contractAddress,
      record,
      config,
      register,
      transfer,
    };
  }

  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: 'arraybuffer' })
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`)
    }
    return r.data
  }

  const upload = async (): Promise<number> => {
    const meta = {
      source: "https://github.com/CosmWasm/cosmwasm-examples/tree/nameservice-0.11.0/nameservice",
      builder: "cosmwasm/rust-optimizer:0.11.5"
    };
    const sourceUrl = "https://github.com/CosmWasm/cosmwasm-examples/releases/download/nameservice-0.11.0/contract.wasm";
    const wasm = await downloadWasm(sourceUrl);
    const result = await client.upload(wasm, meta);
    return result.codeId;
  }

  const instantiate = async (codeId: number, initMsg: Record<string, unknown>, label: string): Promise<NameServiceInstance> => {
    const result = await client.instantiate(codeId, initMsg, label, { memo: `Init ${label}`});
    return use(result.contractAddress);
  }

  return { upload, instantiate, use };
}

// Demo:
// const client = await useOptions(coralnetOptions).setup(PASSWORD);
// const { address } = await client.getAccount()
// const factory = NameService(client)
//
// const codeId = await factory.upload();
// codeId -> 12
// const initMsg = { purchase_price: { denom: "ushell", amount:"1000" }, transfer_price: { denom: "ushell", amount:"1000" }}
// const contract = await factory.instantiate(12, initMsg, "My Name Service")
// contract.contractAddress -> 'coral1267wq2zk22kt5juypdczw3k4wxhc4z47mug9fd'
//
// OR
//
// const contract = factory.use('coral1267wq2zk22kt5juypdczw3k4wxhc4z47mug9fd')
//
// const randomAddress = 'coral162d3zk45ufaqke5wgcd3kh336k6p3kwwkdj3ma'
//
// contract.config()
// contract.register("name", [{"denom": "ushell", amount: "4000" }])
// contract.record("name")
// contract.transfer("name", randomAddress, [{"denom": "ushell", amount: "2000" }])
//
