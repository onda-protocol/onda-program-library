import * as anchor from "@project-serum/anchor";
import * as borsh from "@coral-xyz/borsh";
import {
  getConcurrentMerkleTreeAccountSize,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import assert from "assert";
import base58 from "bs58";
import { OndaSocial } from "../../target/types/onda_social";
import { requestAirdrop } from "../helpers";

const program = anchor.workspace.OndaSocial as anchor.Program<OndaSocial>;
const connection = program.provider.connection;

function findForumConfigPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [merkleTree.toBuffer()],
    program.programId
  )[0];
}

function findEntryId(merkleTree: anchor.web3.PublicKey, entryIndex: number) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("entry"),
      merkleTree.toBuffer(),
      new anchor.BN(entryIndex).toBuffer("le"),
    ],
    program.programId
  )[0];
}

describe.only("Onda social", () => {
  const maxDepth = 14;
  const maxBufferSize = 64;
  const payer = program.provider.publicKey;
  const merkleTreeKeypair = anchor.web3.Keypair.generate();
  const merkleTree = merkleTreeKeypair.publicKey;
  const forumConfig = findForumConfigPda(merkleTree);

  it("Creates a new tree", async () => {
    const space = getConcurrentMerkleTreeAccountSize(maxDepth, maxBufferSize);
    const lamports = await connection.getMinimumBalanceForRentExemption(space);
    console.log("Allocating ", space, " bytes for merkle tree");
    console.log(lamports, " lamports required for rent exemption");
    console.log(
      lamports / anchor.web3.LAMPORTS_PER_SOL,
      " SOL required for rent exemption"
    );
    const allocTreeIx = anchor.web3.SystemProgram.createAccount({
      lamports,
      space,
      fromPubkey: payer,
      newAccountPubkey: merkleTree,
      programId: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    });

    const createPostIx = await program.methods
      .initForum(maxDepth, maxBufferSize, {
        // collection: { collection: anchor.web3.Keypair.generate().publicKey },
        none: {},
      })
      .accounts({
        payer,
        forumConfig,
        merkleTree,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
      })
      .instruction();

    const tx = new anchor.web3.Transaction().add(allocTreeIx).add(createPostIx);
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

  it("Adds a post to the tree", async () => {
    const mdx = `
    # My MDX Example
    
    This is an example paragraph in an MDX file. You can include **bold** and *italic* text, just like in regular Markdown.
    `;

    const entryId = findEntryId(merkleTree, 0);
    console.log("Entry ID: ", entryId.toBase58());
    const signature = await program.methods
      .addEntry({ data: { textPost: { title: "Hello World!", body: mdx } } })
      .accounts({
        forumConfig,
        merkleTree,
        author: payer,
        mint: null,
        tokenAccount: null,
        metadata: null,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" });

    const parsedTx = await program.provider.connection.getParsedTransaction(
      signature,
      "confirmed"
    );
    const innerInstructions = parsedTx.meta.innerInstructions[0];
    const noopIx = innerInstructions.instructions[2];
    if ("data" in noopIx) {
      const serializedData = noopIx.data;
      const data = base58.decode(serializedData);
      console.log(data);
      // id: Pubkey,
      // entry_type: EntryType,
      // author: Pubkey,
      // created_at: i64,
      // edited_at: Option<i64>,
      // nonce: u64,
      // data_hash: [u8; 32],

      interface Leaf {
        id: anchor.web3.PublicKey;
        // entry_type: number;
        // author: anchor.web3.PublicKey;
        // created_at: number;
        // edited_at: number | null;
        // nonce: number;
        // data_hash: Buffer;
      }

      const layout = borsh.struct(
        [
          borsh.publicKey("id"),
          // borsh.u8("entry_type"),
          // borsh.publicKey("author"),
          // borsh.i64("created_at"),
          // borsh.option(borsh.i64("edited_at"), "edited_at"),
          // borsh.u64("nonce"),
          // borsh.vec(borsh.u8("data"), "data_hash"),
        ],
        "Leaf"
      );
      const decoded: Leaf = layout.decode(data.slice(8));
      console.log("Decoded: ", {
        id: decoded.id.toBase58(),
      });
    }
    assert.ok(true);
  });
});
