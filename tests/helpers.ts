import * as anchor from "@project-serum/anchor";

export async function requestAirdrop(
  connection: anchor.web3.Connection,
  publicKey: anchor.web3.PublicKey
): Promise<void> {
  const blockhashWithExpiryBlockHeight = await connection.getLatestBlockhash();
  const signature = await connection.requestAirdrop(
    publicKey,
    anchor.web3.LAMPORTS_PER_SOL * 10
  );
  await connection.confirmTransaction({
    signature,
    ...blockhashWithExpiryBlockHeight,
  });
}
