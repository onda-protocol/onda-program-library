import assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as helpers from "./helpers";
import { PROGRAM_ID as METADATA_PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "@metaplex-foundation/mpl-bubblegum";
import {
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";

describe.only("Awards", () => {
  it("Creates a new award", async () => {
    const authority = anchor.web3.Keypair.generate();

    await helpers.requestAirdrop(authority.publicKey);
    await helpers.createAward(authority);
  });

  it("Mints a reward to the provied address", async () => {
    const amount = anchor.web3.LAMPORTS_PER_SOL / 100;
    const authority = anchor.web3.Keypair.generate();
    const treasury = anchor.web3.Keypair.generate().publicKey;
    const entryId = anchor.web3.Keypair.generate().publicKey;
    const recipient = anchor.web3.Keypair.generate().publicKey;
    const program = await helpers.getAwardsProgram(authority);

    await helpers.requestAirdrop(authority.publicKey);
    const accounts = await helpers.createAward(authority, treasury, amount, {
      receipt: {},
    });
    const bubblegumSignerPda = await helpers.findBubblegumSignerPda();

    const awardIx = await program.methods
      .giveAward(null)
      .accounts({
        entryId,
        treasury,
        recipient,
        payer: authority.publicKey,
        additionalSigner: null,
        award: accounts.awardPda,
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
      .instruction();

    const modifyComputeUnits =
      anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
        units: 250_000,
      });
    const addPriorityFee = anchor.web3.ComputeBudgetProgram.setComputeUnitPrice(
      {
        microLamports: 1,
      }
    );

    const tx = new anchor.web3.Transaction()
      .add(modifyComputeUnits)
      .add(addPriorityFee)
      .add(awardIx);
    tx.feePayer = authority.publicKey;

    await program.provider.sendAndConfirm(tx, [authority], {
      commitment: "confirmed",
      skipPreflight: true,
    });

    const recipientAccountInfo =
      await program.provider.connection.getAccountInfo(recipient);
    const treasuryAccountInfo =
      await program.provider.connection.getAccountInfo(treasury);

    assert.equal(
      recipientAccountInfo.lamports,
      amount / 2,
      "recipient.balance"
    );
    assert.equal(
      treasuryAccountInfo.lamports,
      amount / 2,
      "recipient.accountSize"
    );
  });
});
