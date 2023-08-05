import assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as helpers from "./helpers";
import {
  ConcurrentMerkleTreeAccount,
  MerkleTree,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";

describe("Moderation", () => {
  it("initializes a team", async () => {
    const admin = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);
    await helpers.initTeam(admin, merkleTree.publicKey);

    const forumConfigPda = helpers.findForumConfigPda(merkleTree.publicKey);
    const forumConfigAccount =
      await helpers.compressionProgram.account.forumConfig.fetch(
        forumConfigPda
      );
    const teamPda = helpers.findTeamPda(merkleTree.publicKey);
    const teamAccount = await helpers.moderationProgram.account.team.fetch(
      teamPda
    );
    assert.ok(forumConfigAccount.admin.equals(teamPda), "forum.admin");
    assert.ok(teamAccount.forum.equals(merkleTree.publicKey), "team.forum");
    assert.ok(teamAccount.members[0].role.owner, "member.role");
    assert.ok(
      teamAccount.members[0].address.equals(admin.publicKey),
      "team.members"
    );
  });

  it("adds a member", async () => {
    const admin = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();
    const member = anchor.web3.Keypair.generate();
    const teamPda = await helpers.findTeamPda(merkleTree.publicKey);

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);
    await helpers.initTeam(admin, merkleTree.publicKey);
    const moderationProgram = await helpers.getModerationProgram(admin);

    await moderationProgram.methods
      .addMember({
        moderator: {},
      })
      .accounts({
        admin: admin.publicKey,
        newMember: member.publicKey,
        team: teamPda,
        merkleTree: merkleTree.publicKey,
      })
      .rpc();

    const teamAccount = await helpers.moderationProgram.account.team.fetch(
      teamPda
    );
    assert.ok(
      teamAccount.members[1].address.equals(member.publicKey),
      "team.members"
    );
  });

  it("remove a member", async () => {
    const admin = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();
    const member = anchor.web3.Keypair.generate();
    const teamPda = await helpers.findTeamPda(merkleTree.publicKey);

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);
    await helpers.initTeam(admin, merkleTree.publicKey);
    const moderationProgram = await helpers.getModerationProgram(admin);

    await moderationProgram.methods
      .addMember({
        moderator: {},
      })
      .accounts({
        admin: admin.publicKey,
        newMember: member.publicKey,
        team: teamPda,
        merkleTree: merkleTree.publicKey,
      })
      .rpc();

    await moderationProgram.methods
      .removeMember()
      .accounts({
        admin: admin.publicKey,
        member: member.publicKey,
        team: teamPda,
        merkleTree: merkleTree.publicKey,
      })
      .rpc();

    const teamAccount = await helpers.moderationProgram.account.team.fetch(
      teamPda
    );
    assert.equal(teamAccount.members[1], undefined, "team.members");
  });

  it("deletes an entry", async () => {
    const admin = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();
    const forumConfigPda = helpers.findForumConfigPda(merkleTree.publicKey);
    const teamPda = await helpers.findTeamPda(merkleTree.publicKey);

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);
    await helpers.initTeam(admin, merkleTree.publicKey);
    const leafEvent = await helpers.addEntry(
      merkleTree.publicKey,
      "test",
      "https://example.com"
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

    const moderationProgram = await helpers.getModerationProgram(admin);
    await moderationProgram.methods
      .deleteEntry(
        Array.from(merkleTreeAccount.getCurrentRoot()),
        leafEvent.createdAt,
        leafEvent.editedAt,
        leafEvent.dataHash,
        leafEvent.nonce,
        leafEvent.nonce.toNumber()
      )
      .accounts({
        member: admin.publicKey,
        team: teamPda,
        author: leafEvent.author,
        forumConfig: forumConfigPda,
        merkleTree: merkleTree.publicKey,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        ondaCompression: helpers.compressionProgram.programId,
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
      .rpc({
        skipPreflight: true,
      });
  });
});
