import * as anchor from "@project-serum/anchor";
import * as helpers from "./helpers";
import { PROGRAM_ID as METADATA_PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "@metaplex-foundation/mpl-bubblegum";
import {
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";

describe.only("Rewards", () => {
  it("Creates a new reward", async () => {
    const authority = anchor.web3.Keypair.generate();

    await helpers.requestAirdrop(authority.publicKey);
    await helpers.createReward(authority);
  });

  it("Mints a reward to the provied address", async () => {
    const authority = anchor.web3.Keypair.generate();
    const entryId = anchor.web3.Keypair.generate().publicKey;
    const program = await helpers.getRewardsProgram(authority);

    await helpers.requestAirdrop(authority.publicKey);
    const accounts = await helpers.createReward(authority);
    const bubblegumSignerPda = await helpers.findBubblegumSignerPda();

    try {
      await program.methods
        .giveReward()
        .accounts({
          payer: authority.publicKey,
          sessionToken: null,
          signer: authority.publicKey,
          reward: accounts.rewardPda,
          leafOwner: entryId,
          merkleTree: accounts.merkleTree,
          treeAuthority: accounts.treeAuthorityPda,
          collectionAuthorityRecordPda: accounts.collectionAuthorityRecordPda,
          collectionMint: accounts.collectionMint,
          collectionMetadata: accounts.collectionMetadata,
          editionAccount: accounts.editionPda,
          logWrapper: SPL_NOOP_PROGRAM_ID,
          bubblegumSigner: bubblegumSignerPda,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          tokenMetadataProgram: METADATA_PROGRAM_ID,
          bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
        })
        .rpc({
          skipPreflight: true,
        });
    } catch (err) {
      console.log(err);
      throw err;
    }
  });
});
