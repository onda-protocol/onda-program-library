import fs from "fs";
import path from "path";
import os from "os";
import { AnchorProvider, Wallet, Program, web3 } from "@project-serum/anchor";
import * as splToken from "@solana/spl-token";
import { OndaBloom, IDL } from "../target/types/onda_bloom";

const MINT = new web3.PublicKey("pktnre2sUNQZXwHicZj6njpShhSazmzQz5rJtcqnkG5");
const PROGRAM_ID = new web3.PublicKey(
  "onda3Sxku2NT88Ho8WfEgbkavNEELWzaguvh4itdn3C"
);

const connection = new web3.Connection(
  "https://rpc.helius.xyz/?api-key=a4184ebc-516f-4432-a449-78b8dd47e6bf"
);

async function main() {
  const keypairJson = fs.readFileSync(
    path.join(os.homedir(), ".config", "solana", "id.json"),
    "utf-8"
  );
  const keypair = web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(keypairJson))
  );
  const program = new Program(
    IDL,
    PROGRAM_ID,
    new AnchorProvider(connection, new Wallet(keypair), {
      preflightCommitment: "confirmed",
    })
  );

  const [rewardTokenAccount] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("reward_escrow"), MINT.toBuffer()],
    PROGRAM_ID
  );

  await program.methods
    .init()
    .accounts({
      mint: MINT,
      rewardTokenAccount,
      signer: keypair.publicKey,
      tokenProgram: splToken.TOKEN_PROGRAM_ID,
      systemProgram: web3.SystemProgram.programId,
    })
    .rpc();
}

main();
