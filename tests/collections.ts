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
  const keypair = anchor.web3.Keypair.generate();
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
    assert.deepEqual(collectonData.config, {
      loanEnabled: true,
      loanBasisPoints: 200,
      optionEnabled: true,
      optionBasisPoints: 200,
      rentalEnabled: true,
      rentalBasisPoints: 200,
    });
  });
});
