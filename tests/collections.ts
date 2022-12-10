require("dotenv").config();

import assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as helpers from "./helpers";
import { NftWithToken } from "@metaplex-foundation/js";
import { DexloanListings } from "../target/types/dexloan_listings";

// Configure the client to use the local cluster.
const connection = new anchor.web3.Connection(
  "http://127.0.0.1:8899",
  anchor.AnchorProvider.defaultOptions().preflightCommitment
);

describe("Collections", async () => {
  let authority: anchor.web3.Keypair;
  let provider: anchor.AnchorProvider;
  let program: anchor.Program<DexloanListings>;

  let collectionPda: anchor.web3.PublicKey;
  let collectionMint: NftWithToken;

  it("Initializes a collection", async () => {
    authority = await helpers.getAuthority();
    provider = helpers.getProvider(connection, authority);
    program = helpers.getProgram(provider);
    await helpers.requestAirdrop(connection, authority.publicKey);
    const { collection } = await helpers.mintNFT(connection, authority);
    collectionMint = collection;
    collectionPda = await helpers.findCollectionAddress(collection.address);

    const collectonData = await program.account.collection.fetch(collectionPda);
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

  it("Updates a collection", async () => {
    const signer = await helpers.getSigner();

    const config = {
      loanEnabled: true,
      loanBasisPoints: 100,
      optionEnabled: true,
      optionBasisPoints: 100,
      rentalEnabled: false,
      rentalBasisPoints: 0,
    };
    await program.methods
      .updateCollection(config)
      .accounts({
        signer: signer.publicKey,
        authority: authority.publicKey,
        collection: collectionPda,
        mint: collectionMint.address,
      })
      .signers([signer])
      .rpc();

    const collectonData = await program.account.collection.fetch(collectionPda);
    assert.deepEqual(collectonData.config, config);
  });

  it("closes a collection", async () => {
    const signer = await helpers.getSigner();

    await program.methods
      .closeCollection()
      .accounts({
        signer: signer.publicKey,
        authority: authority.publicKey,
        collection: collectionPda,
        mint: collectionMint.address,
      })
      .signers([signer])
      .rpc();

    try {
      const collectonData = await program.account.collection.fetch(
        collectionPda
      );
    } catch (err) {
      assert.ok(err.message.includes("Account does not exist"));
    }
  });
});
