import * as anchor from "@project-serum/anchor";
import {
  createVerifyLeafIx,
  ConcurrentMerkleTreeAccount,
  getConcurrentMerkleTreeAccountSize,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import assert from "assert";
import base58 from "bs58";
import { keccak_256 } from "js-sha3";
import { OndaSocial, IDL } from "../target/types/onda_social";
import { requestAirdrop } from "./helpers";

const program = anchor.workspace.OndaSocial as anchor.Program<OndaSocial>;
const connection = program.provider.connection;

type OndaSocialTypes = anchor.IdlTypes<OndaSocial>;
type DataV1 = OndaSocialTypes["DataV1"];
type LeafSchemaV1 = SnakeToCamelCaseObj<OndaSocialTypes["LeafSchema"]["v1"]>;
type SnakeToCamelCase<S extends string> = S extends `${infer T}_${infer U}`
  ? `${T}${Capitalize<SnakeToCamelCase<U>>}`
  : S;
type SnakeToCamelCaseObj<T> = T extends object
  ? {
      [K in keyof T as SnakeToCamelCase<K & string>]: T[K];
    }
  : T;

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
      new anchor.BN(entryIndex).toBuffer("le", 8),
    ],
    program.programId
  )[0];
}

function findLikeRecordPda(
  entryId: anchor.web3.PublicKey,
  author: anchor.web3.PublicKey
) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("likes"), entryId.toBuffer(), author.toBuffer()],
    program.programId
  )[0];
}

describe.only("Onda social", () => {
  const maxDepth = 14;
  const maxBufferSize = 256;
  const payer = program.provider.publicKey;
  const merkleTreeKeypair = anchor.web3.Keypair.generate();
  const merkleTree = merkleTreeKeypair.publicKey;
  const forumConfig = findForumConfigPda(merkleTree);

  let entryData: DataV1;
  let eventData: LeafSchemaV1;

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
    let signature;

    try {
      signature = await program.methods
        .addEntry({
          textPost: {
            title: "Hello World!",
            body: `Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.`,
          },
        })
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
    } catch (err) {
      console.log(err);
      throw err;
    }

    const parsedTx = await program.provider.connection.getParsedTransaction(
      signature,
      "confirmed"
    );
    const innerInstructions = parsedTx.meta.innerInstructions[0];
    const noopIx = innerInstructions.instructions[0];
    if ("data" in noopIx) {
      const serializedEvent = noopIx.data;
      const event = base58.decode(serializedEvent);
      const eventBuffer = Buffer.from(event.slice(8));
      eventData = program.coder.types.decode("LeafSchema", eventBuffer).v1;
      assert.ok(eventData);
    } else {
      assert.fail("No data in noopIx");
    }

    const outerIx = parsedTx.transaction.message.instructions[0];
    if ("data" in outerIx) {
      const data = outerIx.data;
      const entry = base58.decode(data);
      const buffer = Buffer.from(entry.slice(8));
      entryData = program.coder.types.decode("DataV1", buffer);
      assert.ok(entryData);
    } else {
      assert.fail("No data in outerIx");
    }
  });

  it("Allows users to tip the author", async () => {
    const tipper = anchor.web3.Keypair.generate();
    const newProgram = new anchor.Program<OndaSocial>(
      IDL,
      program.programId,
      new anchor.AnchorProvider(
        connection,
        new anchor.Wallet(tipper),
        anchor.AnchorProvider.defaultOptions()
      )
    );
    await requestAirdrop(connection, tipper.publicKey);

    const entryId = findEntryId(merkleTree, 0);
    const likeRecordPda = findLikeRecordPda(entryId, payer);

    await newProgram.methods
      .likeEntry(entryId)
      .accounts({
        payer: tipper.publicKey,
        author: payer,
        forumConfig,
        merkleTree,
        likeRecord: likeRecordPda,
      })
      .rpc();

    const likeRecord = await program.account.likeRecord.fetch(likeRecordPda);

    assert.equal(likeRecord.amount.toNumber(), 1);
  });

  it("Verifies an entry", async () => {
    const merkleTreeAccount =
      await ConcurrentMerkleTreeAccount.fromAccountAddress(
        connection,
        merkleTree
      );
    const leafIndex = new anchor.BN(0);
    const entryId = findEntryId(merkleTree, 0);
    const verifyIx = createVerifyLeafIx(merkleTree, {
      root: merkleTreeAccount.getCurrentRoot(),
      leaf: computeCompressedEntryHash(
        entryId,
        payer,
        eventData.createdAt,
        eventData.editedAt,
        leafIndex,
        {
          textPost: {
            title: "Hello World!",
            body: `Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.`,
          },
        }
      ),
      leafIndex: 0,
      proof: [],
    });

    const tx = new anchor.web3.Transaction().add(verifyIx);
    tx.feePayer = payer;

    try {
      await program.provider.sendAndConfirm(tx, [], {
        commitment: "confirmed",
      });
    } catch (err) {
      console.log(err.logs);
      throw err;
    }
  });
});

function computeDataHash(data: DataV1): Buffer {
  const encoded = program.coder.types.encode<DataV1>("DataV1", data);
  return Buffer.from(keccak_256.digest(encoded));
}

function computeCompressedEntryHash(
  entryId: anchor.web3.PublicKey,
  author: anchor.web3.PublicKey,
  createdAt: anchor.BN,
  editedAt: anchor.BN | null,
  nonce: anchor.BN,
  data: DataV1
): Buffer {
  const message = Buffer.concat([
    Buffer.from([0x1]), // All NFTs are version 1 right now
    entryId.toBuffer(),
    author.toBuffer(),
    createdAt.toBuffer("le", 8),
    new anchor.BN(editedAt || 0).toBuffer("le", 8),
    nonce.toBuffer("le", 8),
    computeDataHash(data),
  ]);

  return Buffer.from(keccak_256.digest(message));
}
