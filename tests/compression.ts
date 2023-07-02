import * as anchor from "@project-serum/anchor";
import {
  createVerifyLeafIx,
  ConcurrentMerkleTreeAccount,
  getConcurrentMerkleTreeAccountSize,
  hash,
  MerkleTree,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import { PublicKey } from "@metaplex-foundation/js";
import assert from "assert";
import base58 from "bs58";
import { keccak_256 } from "js-sha3";
import { OndaCompression, IDL } from "../target/types/onda_compression";
import { requestAirdrop } from "./helpers";

const program = anchor.workspace
  .OndaCompression as anchor.Program<OndaCompression>;
const connection = program.provider.connection;

type OndaCompressionTypes = anchor.IdlTypes<OndaCompression>;
type DataV1 = OndaCompressionTypes["DataV1"];
type LeafSchemaV1 = SnakeToCamelCaseObj<
  OndaCompressionTypes["LeafSchema"]["v1"]
>;
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

async function createAnchorProgram(
  keypair: anchor.web3.Keypair = anchor.web3.Keypair.generate()
) {
  await requestAirdrop(connection, keypair.publicKey);
  return new anchor.Program<OndaCompression>(
    IDL,
    program.programId,
    new anchor.AnchorProvider(
      connection,
      new anchor.Wallet(keypair),
      anchor.AnchorProvider.defaultOptions()
    )
  );
}

describe.only("onda_compression", () => {
  const maxDepth = 20; // 3;
  const maxBufferSize = 64; // 8;
  // Allocation additional bytes for the canopy
  const canopyDepth = 14;
  const canopySpace = (Math.pow(2, canopyDepth) - 2) * 32;
  const merkleTreeKeypair = anchor.web3.Keypair.generate();
  const merkleTree = merkleTreeKeypair.publicKey;
  const forumConfig = findForumConfigPda(merkleTree);

  const authors: anchor.web3.Keypair[] = [];
  const leafSchemaV1: LeafSchemaV1[] = [];
  const dataArgs: DataV1[] = [];

  async function createPost(title: string, uri: string) {
    const author = anchor.web3.Keypair.generate();
    const program = await createAnchorProgram(author);
    return program.methods
      .addEntry({
        textPost: { title, uri },
      })
      .accounts({
        forumConfig,
        merkleTree,
        author: program.provider.publicKey,
        mint: null,
        tokenAccount: null,
        metadata: null,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
      })
      .rpc({ commitment: "confirmed" })
      .then(async (signature) => {
        const parsedTx = await program.provider.connection.getParsedTransaction(
          signature,
          "confirmed"
        );
        const innerInstructions = parsedTx.meta.innerInstructions[0];
        const noopIx = innerInstructions.instructions[0];
        let leafSchema: LeafSchemaV1;
        if ("data" in noopIx) {
          const serializedEvent = noopIx.data;
          const event = base58.decode(serializedEvent);
          const eventBuffer = Buffer.from(event.slice(8));
          leafSchema = program.coder.types.decode("LeafSchema", eventBuffer).v1;
          leafSchemaV1[leafSchema.nonce.toNumber()] = leafSchema;
          authors[leafSchema.nonce.toNumber()] = author;
        } else {
          throw new Error("No data in noopIx");
        }

        const outerIx = parsedTx.transaction.message.instructions[0];
        if ("data" in outerIx) {
          const data = outerIx.data;
          const entry = base58.decode(data);
          const buffer = Buffer.from(entry.slice(8));
          dataArgs[leafSchema.nonce.toNumber()] = program.coder.types.decode(
            "DataV1",
            buffer
          );
        } else {
          throw new Error("No data in outerIx");
        }
      });
  }

  it("Creates a new tree", async () => {
    const program = await createAnchorProgram();
    const payer = program.provider.publicKey;
    const space = getConcurrentMerkleTreeAccountSize(maxDepth, maxBufferSize);
    const totalSpace = space + canopySpace;
    const canopyCost = await connection.getMinimumBalanceForRentExemption(
      canopySpace
    );
    const lamports = await connection.getMinimumBalanceForRentExemption(
      totalSpace
    );
    console.log("Allocating ", totalSpace, " bytes for merkle tree");
    console.log(lamports, " lamports required for rent exemption");
    console.log("Canopy cost: ", canopyCost / anchor.web3.LAMPORTS_PER_SOL);
    console.log(
      lamports / anchor.web3.LAMPORTS_PER_SOL,
      " SOL required for rent exemption"
    );
    const allocTreeIx = anchor.web3.SystemProgram.createAccount({
      lamports,
      space: totalSpace,
      fromPubkey: payer,
      newAccountPubkey: merkleTree,
      programId: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    });

    const initForumIx = await program.methods
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

    const tx = new anchor.web3.Transaction().add(allocTreeIx).add(initForumIx);
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

  it("Adds multiple posts to the tree", async () => {
    try {
      await Promise.all([
        createPost(
          "Hello World 1!",
          "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        ),
        createPost(
          "Hello World 2!",
          "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        ),
        createPost(
          "Hello World 3!",
          "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        ),
        createPost(
          "Hello World 4!",
          "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        ),
        createPost(
          "Hello World 5!",
          "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        ),
        createPost(
          "Hello World 6!",
          "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        ),
      ]);
      assert.ok(true);
    } catch (err) {
      console.log(err);
      throw err;
    }
  });

  it.skip("Verifies an entry", async () => {
    const program = await createAnchorProgram();
    const payer = program.provider.publicKey;
    const merkleTreeAccount =
      await ConcurrentMerkleTreeAccount.fromAccountAddress(
        connection,
        merkleTree
      );

    const leaves = computeLeaves(leafSchemaV1, dataArgs);
    const tree = MerkleTree.sparseMerkleTreeFromLeaves(
      leaves,
      merkleTreeAccount.getMaxDepth()
    );
    const leafIndex = 1;
    const proof = tree.getProof(leafIndex);
    const verifyIx = createVerifyLeafIx(merkleTree, proof);
    const tx = new anchor.web3.Transaction().add(verifyIx);
    tx.feePayer = payer;

    try {
      await program.provider.sendAndConfirm(tx, [], {
        commitment: "confirmed",
        skipPreflight: true,
      });
    } catch (err) {
      console.log(err.logs);
      throw err;
    }
  });

  it("Deletes an entry", async () => {
    const leafIndex = 0;
    console.log("Generating proof for idx ", leafIndex);
    const author = authors[leafIndex];
    const program = await createAnchorProgram(author);

    const merkleTreeAccount =
      await ConcurrentMerkleTreeAccount.fromAccountAddress(
        connection,
        merkleTree
      );
    const leaves = computeLeaves(leafSchemaV1, dataArgs);

    const nodeIndex = getNodeIndexFromLeafIndex(
      leafIndex,
      merkleTreeAccount.getMaxDepth()
    );
    const nodes: Node[] = generatePathNodesFromIndex(
      nodeIndex,
      merkleTreeAccount.getMaxDepth(),
      merkleTreeAccount.getCanopyDepth()
    );
    const leafIndexes = nodes.map((node) => node.getLeafIndexesFromPath());
    console.log("leafIndexes required: ", flatten(leafIndexes).length);

    const proof = leafIndexes
      .map((path) => {
        if (typeof path === "number") {
          const leafHash = leaves[path] ?? Buffer.alloc(32);
          return new PublicKey(leafHash);
        } else {
          return new PublicKey(recursiveHash(leaves, path));
        }
      })
      .map((pubkey) => ({
        pubkey,
        isWritable: false,
        isSigner: false,
      }));
    console.log(
      "here is the proof: ",
      proof.map((n) => n.pubkey.toBase58())
    );

    try {
      /**
       * root: [u8; 32],
       * created_at: i64,
       * edited_at: Option<i64>,
       * data_hash: [u8; 32],
       * nonce: u64,
       * index: u32,
       **/
      await program.methods
        .deleteEntry(
          Array.from(merkleTreeAccount.getCurrentRoot()),
          leafSchemaV1[leafIndex].createdAt,
          leafSchemaV1[leafIndex].editedAt,
          leafSchemaV1[leafIndex].dataHash,
          leafSchemaV1[leafIndex].nonce,
          leafSchemaV1[leafIndex].nonce.toNumber()
        )
        .accounts({
          forumConfig,
          merkleTree,
          author: authors[leafIndex].publicKey,
          logWrapper: SPL_NOOP_PROGRAM_ID,
          compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .remainingAccounts(proof)
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

function flatten(items) {
  const flat = [];

  items.forEach((item) => {
    if (Array.isArray(item)) {
      flat.push(...flatten(item));
    } else {
      flat.push(item);
    }
  });

  return flat;
}

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
    Buffer.from([0x1]), // v1
    entryId.toBuffer(),
    author.toBuffer(),
    createdAt.toBuffer("le", 8),
    new anchor.BN(editedAt || 0).toBuffer("le", 8),
    nonce.toBuffer("le", 8),
    computeDataHash(data),
  ]);

  return Buffer.from(keccak_256.digest(message));
}

type NestedPath = Array<number | NestedPath>;

class Node {
  private _path: NestedPath = [];

  constructor(
    public readonly index: number,
    public readonly maxDepth: number,
    public readonly depth: number
  ) {}

  private _generatePath(nodeIndex: number, depth: number) {
    let path = nodeIndex;

    if (depth === 0) return path;

    let leftChildIndex = 2 * nodeIndex + 2;
    let rightChildIndex = 2 * nodeIndex + 3;

    return [
      this._generatePath(leftChildIndex, depth - 1),
      this._generatePath(rightChildIndex, depth - 1),
    ];
  }

  public generatePath() {
    this._path = this._generatePath(this.index, this.depth);
  }

  private _getLeafIndexesFromPath(path: number | NestedPath) {
    if (typeof path === "number") {
      return getLeafIndexFromNodeIndex(path, this.maxDepth);
    }

    return [
      this._getLeafIndexesFromPath(path[0]),
      this._getLeafIndexesFromPath(path[1]),
    ];
  }

  public getLeafIndexesFromPath(): number | NestedPath {
    return this._getLeafIndexesFromPath(this._path);
  }

  get path() {
    return this._path;
  }
}

function getLeafIndexFromNodeIndex(
  nodeIndex: number,
  maxDepth: number
): number {
  return nodeIndex - (Math.pow(2, maxDepth) - 2);
}

function getNodeIndexFromLeafIndex(leafIndex: number, maxDepth: number) {
  return leafIndex + (Math.pow(2, maxDepth) - 2);
}

function getSiblingIndex(index: number) {
  return index % 2 === 0 ? index + 1 : index - 1;
}

function getParentIndex(index: number) {
  return Math.floor((index - 2) / 2);
}

function generatePathNodesFromIndex(
  index: number,
  maxDepth: number,
  canopyDepth: number,
  depth: number = 0
) {
  const nodes: Node[] = [];
  const siblingIndex = getSiblingIndex(index);
  const node = new Node(siblingIndex, maxDepth, depth);
  node.generatePath();

  nodes.push(node);

  if (depth + 1 < maxDepth - canopyDepth) {
    const parentIndex = getParentIndex(index);
    nodes.push(
      ...generatePathNodesFromIndex(
        parentIndex,
        maxDepth,
        canopyDepth,
        depth + 1
      )
    );
  }

  return nodes;
}

function recursiveHash(leaves: Buffer[], path: NestedPath): Buffer {
  const [left, right] = path;

  if (left instanceof Array && right instanceof Array) {
    return hash(recursiveHash(leaves, left), recursiveHash(leaves, right));
  }

  if (typeof left === "number" && typeof right === "number") {
    const leftBuffer = leaves[left] ?? Buffer.alloc(32);
    const rightBuffer = leaves[right] ?? Buffer.alloc(32);
    return hash(leftBuffer, rightBuffer);
  }

  throw new Error("Invalid path");
}

function computeLeaves(events: LeafSchemaV1[], dataArgs: DataV1[]) {
  const leaves: Buffer[] = [];

  for (const index in events) {
    const entry = events[index];
    const data = dataArgs[entry.nonce.toNumber()];
    const hash = computeCompressedEntryHash(
      entry.id,
      entry.author,
      entry.createdAt,
      entry.editedAt,
      entry.nonce,
      data
    );
    console.log(
      "hash for idx ",
      entry.nonce.toNumber(),
      " is ",
      new PublicKey(hash).toBase58()
    );
    leaves[entry.nonce.toNumber()] = hash;
  }

  return leaves;
}
