require("dotenv").config();

import assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as helpers from "./helpers";

// Configure the client to use the local cluster.
const connection = new anchor.web3.Connection(
  "http://127.0.0.1:8899",
  anchor.AnchorProvider.defaultOptions().preflightCommitment
);

describe("Collections", () => {
  const keypair = anchor.web3.Keypair.fromSecretKey(
    new Uint8Array([
      124, 208, 255, 155, 233, 90, 118, 131, 46, 39, 251, 139, 128, 39, 102, 95,
      152, 29, 11, 251, 94, 142, 210, 207, 43, 45, 190, 97, 177, 241, 91, 213,
      133, 38, 232, 90, 89, 239, 206, 32, 37, 195, 180, 213, 193, 236, 43, 164,
      196, 151, 160, 8, 134, 116, 139, 146, 73, 139, 186, 20, 80, 144, 207, 225,
    ])
  );
  const provider = helpers.getProvider(connection, keypair);
  const program = helpers.getProgram(provider);

  it("Initializes a collection", async () => {
    await helpers.requestAirdrop(connection, keypair.publicKey);
    const { collection } = await helpers.mintNFT(connection, keypair);

    const collectionAddress = await helpers.findCollectionAddress(
      collection.address
    );
    const collectonData = await program.account.collection.fetch(
      collectionAddress
    );

    assert.equal(
      collectonData.mint.toBase58(),
      collection.mint.address.toBase58()
    );
  });
});
