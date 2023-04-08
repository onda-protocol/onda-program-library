import * as anchor from "@project-serum/anchor";
import {
  getConcurrentMerkleTreeAccountSize,
  createVerifyLeafIx,
  ConcurrentMerkleTreeAccount,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
  ValidDepthSizePair,
} from "@solana/spl-account-compression";
import assert from "assert";
import { OndaSocial } from "../../target/types/onda_social";
import { requestAirdrop } from "../helpers";

const program = anchor.workspace.OndaSocial as anchor.Program<OndaSocial>;
const connection = program.provider.connection;

function findTreeAuthorityPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [merkleTree.toBuffer()],
    program.programId
  )[0];
}

describe.only("Onda social", () => {
  it("Creates a new tree", async () => {
    const maxDepth = 14;
    const maxBufferSize = 64;
    const payer = program.provider.publicKey;
    const merkleTreeKeypair = anchor.web3.Keypair.generate();
    const merkleTree = merkleTreeKeypair.publicKey;
    const space = getConcurrentMerkleTreeAccountSize(maxDepth, maxBufferSize);
    const allocTreeIx = anchor.web3.SystemProgram.createAccount({
      fromPubkey: payer,
      newAccountPubkey: merkleTree,
      lamports: await connection.getMinimumBalanceForRentExemption(space),
      space: space,
      programId: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    });

    const treeAuthority = findTreeAuthorityPda(merkleTree);
    const createTreeIx = await program.methods
      .createTree(maxDepth, maxBufferSize)
      .accounts({
        treeAuthority,
        merkleTree,
        payer,
        treeCreator: payer,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
      })
      .instruction();

    const tx = new anchor.web3.Transaction().add(allocTreeIx).add(createTreeIx);
    tx.feePayer = payer;

    await requestAirdrop(connection, payer);

    try {
      await program.provider.sendAndConfirm(tx, [merkleTreeKeypair], {
        commitment: "confirmed",
      });
    } catch (err) {
      console.log(err);
      throw err;
    }

    assert.ok(true);
  });

  it("Adds a post to the tree", async () => {});
});
