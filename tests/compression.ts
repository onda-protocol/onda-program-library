import * as anchor from "@project-serum/anchor";
import {
  createVerifyLeafIx,
  ConcurrentMerkleTreeAccount,
  getConcurrentMerkleTreeAccountSize,
  MerkleTree,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import {
  Metaplex,
  keypairIdentity,
  NftWithToken,
} from "@metaplex-foundation/js";
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

function findProfilePda(author: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("profile"), author.toBuffer()],
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

describe.only("Onda Compression", () => {
  const maxDepth = 14;
  const maxBufferSize = 256;
  const merkleTreeKeypair = anchor.web3.Keypair.generate();
  const merkleTree = merkleTreeKeypair.publicKey;
  const forumConfig = findForumConfigPda(merkleTree);

  const authors: anchor.web3.Keypair[] = [];
  const leafSchemaV1: LeafSchemaV1[] = [];
  const dataArgs: DataV1[] = [];

  async function createPost(title: string, body: string) {
    const author = anchor.web3.Keypair.generate();
    const program = await createAnchorProgram(author);
    authors.push(author);
    return program.methods
      .addEntry({
        textPost: { title, body },
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
        if ("data" in noopIx) {
          const serializedEvent = noopIx.data;
          const event = base58.decode(serializedEvent);
          const eventBuffer = Buffer.from(event.slice(8));
          leafSchemaV1.push(
            program.coder.types.decode("LeafSchema", eventBuffer).v1
          );
        } else {
          throw new Error("No data in noopIx");
        }

        const outerIx = parsedTx.transaction.message.instructions[0];
        if ("data" in outerIx) {
          const data = outerIx.data;
          const entry = base58.decode(data);
          const buffer = Buffer.from(entry.slice(8));
          dataArgs.push(program.coder.types.decode("DataV1", buffer));
        } else {
          throw new Error("No data in outerIx");
        }
      });
  }

  it("Creates a new tree", async () => {
    const program = await createAnchorProgram();
    const payer = program.provider.publicKey;
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
          "Hello World!",
          "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
        ),
        createPost(
          "Hello World 2!",
          "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
        ),
        createPost(
          "Hello World 3!",
          "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
        ),
      ]);
      assert.ok(true);
    } catch (err) {
      console.log(err);
      throw err;
    }
  });

  it("Verifies an entry", async () => {
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
    const leafIndex = 0;
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
    const author = authors[0];
    const program = await createAnchorProgram(author);
    const payer = program.provider.publicKey;

    const dataHash = program.coder.types
      .encode("DataV1", dataArgs[0])
      .map((x) => x);

    await program.methods
      // @ts-expect-error
      .deleteEntry({
        dataHash,
      })
      .accounts({
        forumConfig,
        merkleTree,
        author: author.publicKey,
        logWrapper: SPL_NOOP_PROGRAM_ID,
        compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
      });
  });

  let nft: NftWithToken;
  let user = anchor.web3.Keypair.generate();
  const profilePda = findProfilePda(user.publicKey);

  it("Updates a profile", async () => {
    const program = await createAnchorProgram(user);
    const metaplex = Metaplex.make(connection).use(keypairIdentity(user));
    const result = await metaplex.nfts().create({
      uri: "https://arweave.net/123",
      name: "My NFT",
      sellerFeeBasisPoints: 500,
      creators: [
        {
          address: user.publicKey,
          share: 100,
        },
      ],
    });
    nft = result.nft;

    const accounts = {
      author: user.publicKey,
      profile: profilePda,
      mint: nft.mint.address,
      metadata: nft.metadataAddress,
      tokenAccount: nft.token.address,
    };
    await program.methods.updateProfile("MrGM").accounts(accounts).rpc();

    const profile = await program.account.profile.fetch(profilePda);
    assert.equal(profile.name.replace(/\0/g, ""), "MrGM");
    assert.equal(profile.mint.toBase58(), nft.mint.address.toBase58());
  });

  it("Permissionlessly verifies a profile mint", async () => {
    const accounts = {
      author: user.publicKey,
      profile: profilePda,
      mint: nft.mint.address,
      metadata: nft.metadataAddress,
      tokenAccount: nft.token.address,
    };
    await program.methods.verifyProfile().accounts(accounts).rpc();
    assert.ok(true);
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

function computeLeaves(events: LeafSchemaV1[], dataArgs: DataV1[]) {
  const leaves: Buffer[] = [];

  for (const index in events) {
    const entry = events[index];
    const data = dataArgs[index];
    const hash = computeCompressedEntryHash(
      entry.id,
      entry.author,
      entry.createdAt,
      entry.editedAt,
      entry.nonce,
      data
    );
    leaves[entry.nonce.toNumber()] = hash;
  }

  return leaves;
}
