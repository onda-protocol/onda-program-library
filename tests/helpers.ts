import * as anchor from "@project-serum/anchor";
import { keypairIdentity, Metaplex } from "@metaplex-foundation/js";
import { PROGRAM_ID as METADATA_PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "@metaplex-foundation/mpl-bubblegum";
import {
  getConcurrentMerkleTreeAccountSize,
  SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  SPL_NOOP_PROGRAM_ID,
} from "@solana/spl-account-compression";
import base58 from "bs58";
import { keccak_256 } from "js-sha3";

import {
  OndaCompression,
  IDL as COMPRESSION_IDL,
} from "../target/types/onda_compression";
import {
  OndaModeration,
  IDL as MODERATION_IDL,
} from "../target/types/onda_moderation";
import {
  OndaNamespace,
  IDL as NAMESPACE_IDL,
} from "../target/types/onda_namespace";
import { OndaAwards, IDL as AWARDS_IDL } from "../target/types/onda_awards";

type SnakeToCamelCase<S extends string> = S extends `${infer T}_${infer U}`
  ? `${T}${Capitalize<SnakeToCamelCase<U>>}`
  : S;
type SnakeToCamelCaseObj<T> = T extends object
  ? {
      [K in keyof T as SnakeToCamelCase<K & string>]: T[K];
    }
  : T;
type OndaCompressionTypes = anchor.IdlTypes<OndaCompression>;
export type DataV1 = OndaCompressionTypes["DataV1"];
export type LeafSchemaV1 = SnakeToCamelCaseObj<
  OndaCompressionTypes["LeafSchema"]["v1"]
>;

export const compressionProgram = anchor.workspace
  .OndaCompression as anchor.Program<OndaCompression>;
export const moderationProgram = anchor.workspace
  .OndaModeration as anchor.Program<OndaModeration>;
export const namespaceProgram = anchor.workspace
  .OndaNamespace as anchor.Program<OndaNamespace>;
export const awardsProgram = anchor.workspace
  .OndaAwards as anchor.Program<OndaAwards>;
export const connection = compressionProgram.provider.connection;

export async function requestAirdrop(
  publicKey: anchor.web3.PublicKey
): Promise<void> {
  const blockhash = await connection.getLatestBlockhash();
  const signature = await connection.requestAirdrop(
    publicKey,
    anchor.web3.LAMPORTS_PER_SOL * 10
  );
  await connection.confirmTransaction({
    signature,
    ...blockhash,
  });
}

export async function getCompressionProgram(
  keypair: anchor.web3.Keypair = anchor.web3.Keypair.generate()
) {
  return new anchor.Program<OndaCompression>(
    COMPRESSION_IDL,
    compressionProgram.programId,
    new anchor.AnchorProvider(
      connection,
      new anchor.Wallet(keypair),
      anchor.AnchorProvider.defaultOptions()
    )
  );
}

export async function getModerationProgram(
  keypair: anchor.web3.Keypair = anchor.web3.Keypair.generate()
) {
  return new anchor.Program<OndaModeration>(
    MODERATION_IDL,
    moderationProgram.programId,
    new anchor.AnchorProvider(
      connection,
      new anchor.Wallet(keypair),
      anchor.AnchorProvider.defaultOptions()
    )
  );
}

export async function getNamespaceProgram(
  keypair: anchor.web3.Keypair = anchor.web3.Keypair.generate()
) {
  return new anchor.Program<OndaNamespace>(
    NAMESPACE_IDL,
    namespaceProgram.programId,
    new anchor.AnchorProvider(
      connection,
      new anchor.Wallet(keypair),
      anchor.AnchorProvider.defaultOptions()
    )
  );
}

export async function getAwardsProgram(
  keypair: anchor.web3.Keypair = anchor.web3.Keypair.generate()
) {
  return new anchor.Program<OndaAwards>(
    AWARDS_IDL,
    awardsProgram.programId,
    new anchor.AnchorProvider(
      connection,
      new anchor.Wallet(keypair),
      anchor.AnchorProvider.defaultOptions()
    )
  );
}

export function findForumConfigPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [merkleTree.toBuffer()],
    compressionProgram.programId
  )[0];
}

export function findTeamPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("team"), merkleTree.toBuffer()],
    moderationProgram.programId
  )[0];
}

export function findNamespacePda(name: string) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("namespace"), Buffer.from(name)],
    namespaceProgram.programId
  )[0];
}

export function findTreeMarkerPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("tree_marker"), merkleTree.toBuffer()],
    namespaceProgram.programId
  )[0];
}

export function findAwardPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [merkleTree.toBuffer()],
    awardsProgram.programId
  )[0];
}

export function findTreeAuthorityPda(merkleTree: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [merkleTree.toBuffer()],
    BUBBLEGUM_PROGRAM_ID
  )[0];
}

export function findBubblegumSignerPda() {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("collection_cpi")],
    BUBBLEGUM_PROGRAM_ID
  )[0];
}

export async function initForum(
  admin: anchor.web3.Keypair,
  merkleTree: anchor.web3.Keypair
) {
  const program = await getCompressionProgram(admin);
  const forumConfig = findForumConfigPda(merkleTree.publicKey);
  const maxDepth = 14;
  const bufferSize = 64;
  const canopyDepth = maxDepth - 3;
  const space = getConcurrentMerkleTreeAccountSize(
    maxDepth,
    bufferSize,
    canopyDepth
  );
  const lamports = await connection.getMinimumBalanceForRentExemption(space);
  const allocTreeIx = anchor.web3.SystemProgram.createAccount({
    lamports,
    space: space,
    fromPubkey: admin.publicKey,
    newAccountPubkey: merkleTree.publicKey,
    programId: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  });

  const initForumIx = await program.methods
    .initForum(maxDepth, bufferSize, null)
    .accounts({
      payer: admin.publicKey,
      forumConfig,
      merkleTree: merkleTree.publicKey,
      logWrapper: SPL_NOOP_PROGRAM_ID,
      compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    })
    .instruction();

  const tx = new anchor.web3.Transaction().add(allocTreeIx).add(initForumIx);
  tx.feePayer = admin.publicKey;

  try {
    await program.provider.sendAndConfirm(tx, [merkleTree], {
      commitment: "confirmed",
    });
  } catch (err) {
    console.log(err);
    throw err;
  }
}

export async function addEntry(
  merkleTree: anchor.web3.PublicKey,
  data: DataV1,
  author: anchor.web3.Keypair = anchor.web3.Keypair.generate()
): Promise<LeafSchemaV1> {
  const program = await getCompressionProgram(author);
  const forumConfig = findForumConfigPda(merkleTree);
  await requestAirdrop(author.publicKey);

  return program.methods
    .addEntry(data)
    .accounts({
      forumConfig,
      merkleTree,
      author: program.provider.publicKey,
      sessionToken: null,
      signer: program.provider.publicKey,
      additionalSigner: null,
      mint: null,
      tokenAccount: null,
      metadata: null,
      logWrapper: SPL_NOOP_PROGRAM_ID,
      compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    })
    .rpc({ commitment: "confirmed", skipPreflight: true })
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
      } else {
        throw new Error("No data in noopIx");
      }

      return leafSchema;
    });
}

export async function initTeam(
  admin: anchor.web3.Keypair,
  merkleTree: anchor.web3.PublicKey
) {
  const program = await getModerationProgram(admin);
  const forumConfig = findForumConfigPda(merkleTree);
  const team = findTeamPda(merkleTree);

  return program.methods
    .initialize()
    .accounts({
      team,
      merkleTree,
      forumConfig,
      admin: admin.publicKey,
      ondaCompression: compressionProgram.programId,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc({ commitment: "confirmed", skipPreflight: true });
}

export function computeCompressedEntryHash(
  entryId: anchor.web3.PublicKey,
  author: anchor.web3.PublicKey,
  createdAt: anchor.BN,
  editedAt: anchor.BN | null,
  nonce: anchor.BN,
  dataHash: Buffer
): Buffer {
  const message = Buffer.concat([
    Buffer.from([0x1]), // v1
    entryId.toBuffer(),
    author.toBuffer(),
    createdAt.toBuffer("le", 8),
    new anchor.BN(editedAt || 0).toBuffer("le", 8),
    nonce.toBuffer("le", 8),
    dataHash,
  ]);

  return Buffer.from(keccak_256.digest(message));
}

export async function createAward(authority: anchor.web3.Keypair) {
  const maxDepth = 14;
  const bufferSize = 64;
  const canopyDepth = maxDepth - 3;
  const merkleTree = anchor.web3.Keypair.generate();
  const space = getConcurrentMerkleTreeAccountSize(
    maxDepth,
    bufferSize,
    canopyDepth
  );
  const lamports = await connection.getMinimumBalanceForRentExemption(space);
  const allocTreeIx = anchor.web3.SystemProgram.createAccount({
    lamports,
    space: space,
    fromPubkey: authority.publicKey,
    newAccountPubkey: merkleTree.publicKey,
    programId: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
  });

  const program = await getAwardsProgram(authority);
  const awardPda = findAwardPda(merkleTree.publicKey);
  const treeAuthorityPda = findTreeAuthorityPda(merkleTree.publicKey);

  const metaplex = new Metaplex(connection).use(keypairIdentity(authority));

  const { mintAddress, masterEditionAddress } = await metaplex.nfts().create({
    symbol: "ONDA",
    name: "Onda",
    uri: "https://example.com",
    sellerFeeBasisPoints: 0,
    isCollection: true,
  });

  const metadataPda = await metaplex
    .nfts()
    .pdas()
    .metadata({ mint: mintAddress });
  const collectionAuthorityRecordPda = await metaplex
    .nfts()
    .pdas()
    .collectionAuthorityRecord({
      mint: mintAddress,
      collectionAuthority: awardPda,
    });

  const createRewardIx = await program.methods
    .createAward(maxDepth, bufferSize, {
      symbol: "ONDA",
      name: "Onda",
      uri: "https://example.com",
    })
    .accounts({
      award: awardPda,
      collectionMint: mintAddress,
      collectionMetadata: metadataPda,
      collectionAuthorityRecord: collectionAuthorityRecordPda,
      merkleTree: merkleTree.publicKey,
      treeAuthority: treeAuthorityPda,
      payer: authority.publicKey,
      logWrapper: SPL_NOOP_PROGRAM_ID,
      bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
      tokenMetadataProgram: METADATA_PROGRAM_ID,
      compressionProgram: SPL_ACCOUNT_COMPRESSION_PROGRAM_ID,
    })
    .instruction();

  const tx = new anchor.web3.Transaction().add(allocTreeIx).add(createRewardIx);
  tx.feePayer = authority.publicKey;

  try {
    await program.provider.sendAndConfirm(tx, [merkleTree], {
      commitment: "confirmed",
      skipPreflight: true,
    });
  } catch (err) {
    console.log(err);
    throw err;
  }

  return {
    awardPda,
    treeAuthorityPda,
    collectionAuthorityRecordPda,
    collectionMetadata: metadataPda,
    collectionMint: mintAddress,
    editionPda: masterEditionAddress,
    merkleTree: merkleTree.publicKey,
  };
}
