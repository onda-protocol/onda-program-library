import assert from "assert";
import {
  createVerifyLeafIx,
  ConcurrentMerkleTreeAccount,
  MerkleTree,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import * as anchor from "@project-serum/anchor";
import * as helpers from "./helpers";

describe("Compression", () => {
  it("Inits a forum", async () => {
    const admin = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();
    const forumConfigPda = helpers.findForumConfigPda(merkleTree.publicKey);

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);

    const forumConfigAccount =
      await helpers.compressionProgram.account.forumConfig.fetch(
        forumConfigPda
      );
    assert.ok(forumConfigAccount.admin.equals(admin.publicKey), "forum.admin");
    assert.equal(forumConfigAccount.gate.length, 0, "forum.gate");
  });

  it("Adds a post and comment", async () => {
    const admin = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);
    const leafEvent = await helpers.addEntry(merkleTree.publicKey, {
      textPost: {
        title: "test",
        uri: "https://example.com",
        nsfw: false,
      },
    });
    await helpers.addEntry(merkleTree.publicKey, {
      comment: {
        post: leafEvent.id,
        parent: null,
        uri: "https://example.com",
      },
    });
  });

  it("Verifies an entry", async () => {
    const admin = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);
    const leafEvent = await helpers.addEntry(merkleTree.publicKey, {
      textPost: {
        title: "test",
        uri: "https://example.com",
        nsfw: false,
      },
    });
    const leafHash = helpers.computeCompressedEntryHash(
      leafEvent.id,
      leafEvent.author,
      leafEvent.createdAt,
      leafEvent.editedAt,
      leafEvent.nonce,
      Buffer.from(leafEvent.dataHash)
    );
    const merkleTreeAccount =
      await ConcurrentMerkleTreeAccount.fromAccountAddress(
        helpers.connection,
        merkleTree.publicKey
      );
    const proof = MerkleTree.sparseMerkleTreeFromLeaves(
      [leafHash],
      merkleTreeAccount.getMaxDepth()
    ).getProof(0);

    const verifyIx = createVerifyLeafIx(merkleTree.publicKey, proof);
    const tx = new anchor.web3.Transaction().add(verifyIx);
    tx.feePayer = helpers.compressionProgram.provider.publicKey;
    await helpers.compressionProgram.provider.sendAndConfirm(tx, [], {
      commitment: "confirmed",
      skipPreflight: true,
    });
  });

  it("Deletes an entry", async () => {
    const admin = anchor.web3.Keypair.generate();
    const author = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();
    const forumConfigPda = helpers.findForumConfigPda(merkleTree.publicKey);

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);
    await helpers.initTeam(admin, merkleTree.publicKey);
    const leafEvent = await helpers.addEntry(
      merkleTree.publicKey,
      {
        textPost: {
          title: "test",
          uri: "https://example.com",
          nsfw: false,
        },
      },
      author
    );

    const leafHash = helpers.computeCompressedEntryHash(
      leafEvent.id,
      leafEvent.author,
      leafEvent.createdAt,
      leafEvent.editedAt,
      leafEvent.nonce,
      Buffer.from(leafEvent.dataHash)
    );
    const merkleTreeAccount =
      await ConcurrentMerkleTreeAccount.fromAccountAddress(
        helpers.connection,
        merkleTree.publicKey
      );
    const proof = MerkleTree.sparseMerkleTreeFromLeaves(
      [leafHash],
      merkleTreeAccount.getMaxDepth()
    ).getProof(0);

    try {
      /**
       * root: [u8; 32],
       * created_at: i64,
       * edited_at: Option<i64>,
       * data_hash: [u8; 32],
       * nonce: u64,
       * index: u32,
       **/
      const program = await helpers.getCompressionProgram(author);
      await program.methods
        .deleteEntry(
          Array.from(merkleTreeAccount.getCurrentRoot()),
          leafEvent.createdAt,
          leafEvent.editedAt,
          leafEvent.dataHash,
          leafEvent.nonce,
          leafEvent.nonce.toNumber()
        )
        .accounts({
          forumConfig: forumConfigPda,
          merkleTree: merkleTree.publicKey,
          author: leafEvent.author,
          logWrapper: SPL_NOOP_PROGRAM_ID,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .remainingAccounts(
          proof.proof.map((pubkey) => ({
            pubkey: new anchor.web3.PublicKey(pubkey),
            isSigner: false,
            isWritable: false,
          }))
        )
        .preInstructions([
          anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
            units: 1000000,
          }),
        ])
        .rpc({
          commitment: "confirmed",
          skipPreflight: true,
        });
    } catch (err) {
      console.log(err);
      throw err;
    }
  });
});
