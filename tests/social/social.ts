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
import { IDL, OndaSocial } from "../../target/types/onda_social";
import { requestAirdrop } from "../helpers";

const connection = new anchor.web3.Connection(
  "http://127.0.0.1:8899",
  anchor.AnchorProvider.defaultOptions().preflightCommitment
);

const PROGRAM_ID = new anchor.web3.PublicKey(
  "EF5T6akPE1MuvKHNjD1ZNFz71MbZPDBxF3NN1wPAY1XP"
);
const keypair = anchor.web3.Keypair.generate();
const wallet = new anchor.Wallet(keypair);
const provider = new anchor.AnchorProvider(
  connection,
  wallet,
  anchor.AnchorProvider.defaultOptions()
);
const program = new anchor.Program<OndaSocial>(IDL, PROGRAM_ID, provider);

function findTreeAuthorityPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [merkleTree.toBuffer()],
    PROGRAM_ID
  )[0];
}

describe.only("Onda social", () => {
  it("Creates a new tree", async () => {
    const maxDepth = 14;
    const maxBufferSize = 64;
    const payer = keypair.publicKey;
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
      .createTree(3, 3)
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

    await requestAirdrop(connection, keypair.publicKey);

    try {
      await anchor.web3.sendAndConfirmTransaction(
        connection,
        tx,
        [merkleTreeKeypair, keypair],
        {
          commitment: "confirmed",
        }
      );
    } catch (err) {
      console.log(err);
      throw err;
    }

    assert.ok(true);
  });

  it("Adds a post to the tree", async () => {});
});
