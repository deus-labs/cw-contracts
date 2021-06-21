import { Bip39, Random } from "@cosmjs/crypto";
import {
  DirectSecp256k1HdWallet,
  OfflineDirectSigner,
} from "@cosmjs/proto-signing";
import { makeCosmoshubPath } from "@cosmjs/stargate";
import { NetworkConfig } from "../config/network";

function loadOrCreateMnemonic(): string {
  const storedWalletKey = "mnemonic";

  const stored = localStorage.getItem(storedWalletKey);
  if (stored) {
    return stored;
  }

  const generated = Bip39.encode(Random.getBytes(16)).toString();
  localStorage.setItem(storedWalletKey, generated);
  return generated;
}

export async function loadOrCreateWallet({
  addressPrefix,
}: NetworkConfig): Promise<OfflineDirectSigner> {
  const mnemonic = loadOrCreateMnemonic();
  const options = { hdPaths: [makeCosmoshubPath(0)], prefix: addressPrefix };

  const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, options);
  return wallet;
}
