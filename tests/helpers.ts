import * as anchor from "@project-serum/anchor";
import * as splToken from "@solana/spl-token";
import { IDL, DexloanListings } from "../target/types/dexloan_listings";

const PROGRAM_ID = new anchor.web3.PublicKey(
  "H6FCxCy2KCPJwCoUb9eQCSv41WZBKQaYfB6x5oFajzfj"
);

export function getProgram(
  provider: anchor.AnchorProvider
): anchor.Program<DexloanListings> {
  return new anchor.Program(IDL, PROGRAM_ID, provider);
}

export function getProvider(
  connection: anchor.web3.Connection,
  keypair: anchor.web3.Keypair
): anchor.AnchorProvider {
  const wallet = new anchor.Wallet(keypair);
  return new anchor.AnchorProvider(
    connection,
    wallet,
    anchor.AnchorProvider.defaultOptions()
  );
}

export async function requestAirdrop(
  connection: anchor.web3.Connection,
  publicKey: anchor.web3.PublicKey
): Promise<void> {
  const blockhashWithExpiryBlockHeight = await connection.getLatestBlockhash();
  const signature = await connection.requestAirdrop(
    publicKey,
    anchor.web3.LAMPORTS_PER_SOL * 20
  );
  await connection.confirmTransaction({
    signature,
    ...blockhashWithExpiryBlockHeight,
  });
}

export async function mintNFT(
  connection: anchor.web3.Connection,
  keypair: anchor.web3.Keypair
): Promise<{
  mint: anchor.web3.PublicKey;
  associatedAddress: anchor.web3.PublicKey;
}> {
  // Create the Mint Account for the NFT
  const mint = await splToken.createMint(
    connection,
    keypair,
    keypair.publicKey,
    null,
    0
  );

  const associatedAddress = await splToken.getOrCreateAssociatedTokenAccount(
    connection,
    keypair,
    mint,
    keypair.publicKey
  );

  await splToken.mintTo(
    connection,
    keypair,
    mint,
    associatedAddress.address,
    keypair,
    1
  );

  // Reset mint_authority to null from the user to prevent further minting
  await splToken.setAuthority(
    connection,
    keypair,
    mint,
    keypair.publicKey,
    0,
    null
  );

  return { mint, associatedAddress: associatedAddress.address };
}

export async function findListingAddress(
  mint: anchor.web3.PublicKey,
  borrower: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [listingAccount] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("listing"), mint.toBuffer(), borrower.toBuffer()],
    PROGRAM_ID
  );

  return listingAccount;
}

export async function findLoanAddress(
  mint: anchor.web3.PublicKey,
  borrower: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [listingAccount] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("loan"), mint.toBuffer(), borrower.toBuffer()],
    PROGRAM_ID
  );

  return listingAccount;
}

export async function findCallOptionAddress(
  mint: anchor.web3.PublicKey,
  seller: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [callOptionAccount] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("call_option"), mint.toBuffer(), seller.toBuffer()],
    PROGRAM_ID
  );

  return callOptionAccount;
}

export async function initLoan(
  connection: anchor.web3.Connection,
  options: {
    amount: number;
    basisPoints: number;
    duration: number;
  }
) {
  const keypair = anchor.web3.Keypair.generate();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);

  await requestAirdrop(connection, keypair.publicKey);

  const { mint, associatedAddress } = await mintNFT(connection, keypair);

  const loanAccount = await findLoanAddress(mint, keypair.publicKey);

  const [escrowAccount] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("escrow"), mint.toBuffer()],
    program.programId
  );

  const amount = new anchor.BN(options.amount);
  const basisPoints = new anchor.BN(options.basisPoints);
  const duration = new anchor.BN(options.duration);

  try {
    await program.methods
      .initLoan(amount, basisPoints, duration)
      .accounts({
        mint,
        escrowAccount,
        loanAccount,
        borrower: keypair.publicKey,
        depositTokenAccount: associatedAddress,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
  } catch (error) {
    console.log(error.logs);
    throw error;
  }

  return {
    mint,
    keypair,
    provider,
    program,
    loanAccount,
    escrowAccount,
    associatedAddress,
  };
}

export async function createLoan(connection: anchor.web3.Connection, borrower) {
  const keypair = anchor.web3.Keypair.generate();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  try {
    await program.methods
      .makeLoan()
      .accounts({
        loanAccount: borrower.loanAccount,
        borrower: borrower.keypair.publicKey,
        lender: keypair.publicKey,
        mint: borrower.mint,
        escrowAccount: borrower.escrowAccount,
        depositTokenAccount: borrower.associatedAddress,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .rpc();
  } catch (error) {
    console.log(error.logs);
    throw error;
  }

  return {
    keypair,
    provider,
    program,
  };
}

export async function initCallOption(
  connection: anchor.web3.Connection,
  options: {
    amount: number;
    strikePrice: number;
    expiry: number;
  }
) {
  const keypair = anchor.web3.Keypair.generate();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const { mint, associatedAddress } = await mintNFT(connection, keypair);

  const callOptionAccount = await findCallOptionAddress(
    mint,
    keypair.publicKey
  );

  const [escrowAccount] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("escrow"), mint.toBuffer()],
    program.programId
  );
  const amount = new anchor.BN(options.amount);
  const strikePrice = new anchor.BN(options.strikePrice);
  const expiry = new anchor.BN(options.expiry);

  try {
    await program.methods
      .initCallOption(amount, strikePrice, expiry)
      .accounts({
        mint,
        escrowAccount,
        callOptionAccount,
        seller: keypair.publicKey,
        depositTokenAccount: associatedAddress,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .rpc();
  } catch (error) {
    console.log(error.logs);
    throw error;
  }

  return {
    mint,
    keypair,
    provider,
    program,
    callOptionAccount,
    escrowAccount,
    associatedAddress,
  };
}

export async function buyCallOption(
  connection: anchor.web3.Connection,
  seller
) {
  const keypair = anchor.web3.Keypair.generate();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const associatedAddress = await splToken.getOrCreateAssociatedTokenAccount(
    connection,
    keypair,
    seller.mint,
    keypair.publicKey
  );

  try {
    await program.methods
      .buyCallOption()
      .accounts({
        callOptionAccount: seller.callOptionAccount,
        seller: seller.keypair.publicKey,
        buyer: keypair.publicKey,
        mint: seller.mint,
        escrowAccount: seller.escrowAccount,
        depositTokenAccount: seller.associatedAddress,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .rpc();
  } catch (error) {
    console.log(error.logs);
    throw error;
  }

  return {
    keypair,
    provider,
    program,
    associatedAddress: associatedAddress.address,
  };
}
