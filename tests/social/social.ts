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

function findPostConfigPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [merkleTree.toBuffer()],
    program.programId
  )[0];
}

describe.only("Onda social", () => {
  const maxDepth = 14;
  const maxBufferSize = 64;
  const payer = program.provider.publicKey;
  const merkleTreeKeypair = anchor.web3.Keypair.generate();
  const merkleTree = merkleTreeKeypair.publicKey;
  const postConfig = findPostConfigPda(merkleTree);

  it("Creates a new tree", async () => {
    const space = getConcurrentMerkleTreeAccountSize(maxDepth, maxBufferSize);
    const allocTreeIx = anchor.web3.SystemProgram.createAccount({
      fromPubkey: payer,
      newAccountPubkey: merkleTree,
      lamports: await connection.getMinimumBalanceForRentExemption(space),
      space: space,
      programId: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    });

    const createPostIx = await program.methods
      .createPost({
        maxDepth,
        maxBufferSize,
        collection: anchor.web3.Keypair.generate().publicKey,
        postData: {
          text: {
            title: "Hello world!",
            body: "I should be a markdown string",
          },
        },
      })
      .accounts({
        author: payer,
        postConfig,
        merkleTree,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
      })
      .instruction();

    const tx = new anchor.web3.Transaction().add(allocTreeIx).add(createPostIx);
    tx.feePayer = payer;

    await requestAirdrop(connection, payer);
    await program.provider.sendAndConfirm(tx, [merkleTreeKeypair], {
      commitment: "confirmed",
    });

    assert.ok(true);
  });

  it("Adds a post to the tree", async () => {
    const mdx = `
    # My MDX Example
    
    This is an example paragraph in an MDX file. You can include **bold** and *italic* text, just like in regular Markdown.
    `;

    await program.methods.addComment({ body: mdx }).accounts({
      postConfig,
      merkleTree,
      author: payer,
      logWrapper: SPL_NOOP_PROGRAM_ID,
      compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    });

    assert.ok(true);
  });
});
