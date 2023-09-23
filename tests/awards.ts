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
    const accounts = await helpers.createAward(authority, treasury, amount);
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
        claim: null,
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

  it("Creates a matching award", async () => {
    const authority = anchor.web3.Keypair.generate();

    await helpers.requestAirdrop(authority.publicKey);
    const award = await helpers.createAward(authority);
    const { awardPda } = await helpers.createAward(
      authority,
      undefined,
      undefined,
      award.awardPda
    );
    const program = await helpers.getAwardsProgram(authority);
    const awardWithMatching = await program.account.award.fetch(awardPda);

    assert.ok(
      awardWithMatching.matching.award.equals(award.awardPda),
      "matching award"
    );
  });

  it("Creates a claim when minting a matching award", async () => {
    const amount = anchor.web3.LAMPORTS_PER_SOL / 100;
    const authority = anchor.web3.Keypair.generate();
    const treasury = anchor.web3.Keypair.generate().publicKey;
    const entryId = anchor.web3.Keypair.generate().publicKey;
    const recipient = anchor.web3.Keypair.generate().publicKey;
    const program = await helpers.getAwardsProgram(authority);

    await helpers.requestAirdrop(authority.publicKey);

    const matchingAward = await helpers.createAward(
      authority,
      treasury,
      amount
    );
    const award = await helpers.createAward(
      authority,
      treasury,
      amount,
      matchingAward.awardPda
    );
    const claimPda = await helpers.findClaimPda(
      matchingAward.awardPda,
      recipient
    );

    const bubblegumSignerPda = await helpers.findBubblegumSignerPda();

    await program.methods
      .giveAward(null)
      .accounts({
        entryId,
        treasury,
        recipient,
        payer: authority.publicKey,
        additionalSigner: null,
        award: award.awardPda,
        claim: claimPda,
        merkleTree: award.merkleTree,
        treeAuthority: award.treeAuthorityPda,
        collectionAuthorityRecordPda: award.collectionAuthorityRecordPda,
        collectionMint: award.collectionMint,
        collectionMetadata: award.collectionMetadata,
        editionAccount: award.editionPda,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        bubblegumSigner: bubblegumSignerPda,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
        tokenMetadataProgram: METADATA_PROGRAM_ID,
        bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
      })
      .rpc();

    const claim = await program.account.claim.fetch(claimPda);
    assert.equal(claim.amount, 1, "claim.amount");
  });

  it.only("Allows the recipient to claim an award", async () => {
    const amount = anchor.web3.LAMPORTS_PER_SOL / 100;
    const authority = anchor.web3.Keypair.generate();
    const treasury = anchor.web3.Keypair.generate().publicKey;
    const entryId = anchor.web3.Keypair.generate().publicKey;
    const recipient = anchor.web3.Keypair.generate();
    const program = await helpers.getAwardsProgram(authority);

    await helpers.requestAirdrop(authority.publicKey);
    await helpers.requestAirdrop(recipient.publicKey);

    const matchingAward = await helpers.createAward(
      authority,
      treasury,
      amount
    );
    const award = await helpers.createAward(
      authority,
      treasury,
      amount,
      matchingAward.awardPda
    );
    const claimPda = await helpers.findClaimPda(
      matchingAward.awardPda,
      recipient.publicKey
    );

    const bubblegumSignerPda = await helpers.findBubblegumSignerPda();

    await program.methods
      .giveAward(null)
      .accounts({
        entryId,
        treasury,
        recipient: recipient.publicKey,
        payer: authority.publicKey,
        additionalSigner: null,
        award: award.awardPda,
        claim: claimPda,
        merkleTree: award.merkleTree,
        treeAuthority: award.treeAuthorityPda,
        collectionAuthorityRecordPda: award.collectionAuthorityRecordPda,
        collectionMint: award.collectionMint,
        collectionMetadata: award.collectionMetadata,
        editionAccount: award.editionPda,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        bubblegumSigner: bubblegumSignerPda,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
        tokenMetadataProgram: METADATA_PROGRAM_ID,
        bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
      })
      .rpc();

    const newProgram = await helpers.getAwardsProgram(recipient);
    await newProgram.methods
      .claimAward()
      .accounts({
        treasury,
        recipient: recipient.publicKey,
        award: matchingAward.awardPda,
        claim: claimPda,
        merkleTree: matchingAward.merkleTree,
        treeAuthority: matchingAward.treeAuthorityPda,
        collectionAuthorityRecordPda:
          matchingAward.collectionAuthorityRecordPda,
        collectionMint: matchingAward.collectionMint,
        collectionMetadata: matchingAward.collectionMetadata,
        editionAccount: matchingAward.editionPda,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        bubblegumSigner: bubblegumSignerPda,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
        tokenMetadataProgram: METADATA_PROGRAM_ID,
        bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
      })
      .rpc();

    // Assert claim account has been closed
    const claimAccountInfo = await program.provider.connection.getAccountInfo(
      claimPda
    );
    console.log(claimAccountInfo);
    assert.equal(claimAccountInfo, null, "claimAccountInfo");
  });
});
