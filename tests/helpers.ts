import * as anchor from "@project-serum/anchor";
import * as splToken from "@solana/spl-token";
import * as bip39 from "bip39";
import { derivePath } from "ed25519-hd-key";
import { Metaplex, keypairIdentity } from "@metaplex-foundation/js";
import {
  Metadata,
  PROGRAM_ID as METADATA_PROGRAM_ID,
} from "@metaplex-foundation/mpl-token-metadata";
import { IDL, DexloanListings } from "../target/types/dexloan_listings";

const PROGRAM_ID = new anchor.web3.PublicKey(
  "GDNxgyEcP6b2FtTtCGrGhmoy5AQEiwuv26hV1CLmL1yu"
);

async function fromMnemomic(mnemomic: string) {
  const path = "m/44'/501'/0'/0'";
  const seed = await bip39.mnemonicToSeed(mnemomic);
  const derivedSeed = derivePath(path, seed.toString("hex")).key;
  const keypair = anchor.web3.Keypair.fromSeed(derivedSeed);
  return keypair;
}

export async function getSigner() {
  const mnemomic = process.env.SIGNER_SEED_PHRASE as string;
  return fromMnemomic(mnemomic);
}

export async function getAuthority() {
  const mnemomic = process.env.AUTHORITY_SEED_PHRASE as string;
  return fromMnemomic(mnemomic);
}

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
    anchor.web3.LAMPORTS_PER_SOL * 2
  );
  await connection.confirmTransaction({
    signature,
    ...blockhashWithExpiryBlockHeight,
  });
}

export async function findCollectionAddress(
  collection: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [collectionAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("collection"), collection.toBuffer()],
    PROGRAM_ID
  );

  return collectionAddress;
}

export async function findTokenManagerAddress(
  mint: anchor.web3.PublicKey,
  issuer: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [tokenManagerAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("token_manager"), mint.toBuffer(), issuer.toBuffer()],
    PROGRAM_ID
  );

  return tokenManagerAddress;
}

export async function findLoanAddress(
  mint: anchor.web3.PublicKey,
  borrower: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [loanAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("loan"), mint.toBuffer(), borrower.toBuffer()],
    PROGRAM_ID
  );

  return loanAddress;
}

export async function findLoanOfferAddress(
  collectionMint: anchor.web3.PublicKey,
  lender: anchor.web3.PublicKey,
  id: number
): Promise<anchor.web3.PublicKey> {
  const [loanOfferAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("loan_offer"),
      collectionMint.toBuffer(),
      lender.toBuffer(),
      new anchor.BN(id).toArrayLike(Buffer),
    ],
    PROGRAM_ID
  );

  return loanOfferAddress;
}

export async function findLoanOfferVaultAddress(
  loanOffer: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [loanOfferVaultAddress] =
    await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("loan_offer_vault"), loanOffer.toBuffer()],
      PROGRAM_ID
    );

  return loanOfferVaultAddress;
}

export async function findCallOptionBidAddress(
  collectionMint: anchor.web3.PublicKey,
  buyer: anchor.web3.PublicKey,
  id: number
) {
  const [callOptionBidAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("call_option_bid"),
      collectionMint.toBuffer(),
      buyer.toBuffer(),
      new anchor.BN(id).toArrayLike(Buffer),
    ],
    PROGRAM_ID
  );

  return callOptionBidAddress;
}

export async function findCallOptionBidVaultAddress(
  callOptionBid: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [callOptionBidAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("call_option_bid_vault"), callOptionBid.toBuffer()],
    PROGRAM_ID
  );

  return callOptionBidAddress;
}

export async function findCallOptionAddress(
  mint: anchor.web3.PublicKey,
  seller: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [callOptionAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("call_option"), mint.toBuffer(), seller.toBuffer()],
    PROGRAM_ID
  );

  return callOptionAddress;
}

export async function findRentalAddress(
  mint: anchor.web3.PublicKey,
  lender: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [rentalAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("rental"), mint.toBuffer(), lender.toBuffer()],
    PROGRAM_ID
  );

  return rentalAddress;
}

export async function findRentalEscrowAddress(
  mint: anchor.web3.PublicKey,
  lender: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> {
  const [rentalEscrowAddress] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("rental_escrow"), mint.toBuffer(), lender.toBuffer()],
    PROGRAM_ID
  );

  return rentalEscrowAddress;
}

export async function findMetadataAddress(mint: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from("metadata"), METADATA_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    METADATA_PROGRAM_ID
  );
}

export async function mintNFT(
  connection: anchor.web3.Connection,
  keypair: anchor.web3.Keypair
) {
  const authority = await getAuthority();
  const signer = await getSigner();
  const provider = getProvider(connection, authority);
  const program = getProgram(provider);
  await requestAirdrop(connection, authority.publicKey);

  const metaplex = Metaplex.make(connection).use(keypairIdentity(authority));

  const { nft: collection } = await metaplex
    .nfts()
    .create({
      uri: "https://arweave.net/123",
      name: "My Collection",
      sellerFeeBasisPoints: 500,
      creators: [
        {
          address: authority.publicKey,
          share: 100,
        },
      ],
      isCollection: true,
      collectionIsSized: true,
    })
    .run();

  const collectionAddress = await findCollectionAddress(
    collection.mint.address
  );

  await program.methods
    .initCollection({
      loanEnabled: true,
      optionEnabled: true,
      rentalEnabled: true,
      loanBasisPoints: 200,
      optionBasisPoints: 200,
      rentalBasisPoints: 200,
    })
    .accounts({
      signer: signer.publicKey,
      authority: authority.publicKey,
      collection: collectionAddress,
      mint: collection.address,
    })
    .signers([signer])
    .rpc();

  const { nft } = await metaplex
    .nfts()
    .create({
      uri: "https://arweave.net/123",
      name: "My NFT",
      sellerFeeBasisPoints: 500,
      creators: [
        {
          address: authority.publicKey,
          share: 100,
        },
      ],
      collection: collection.mint.address,
    })
    .run();

  const verifyResult = await metaplex
    .nfts()
    .verifyCollection({
      mintAddress: nft.mint.address,
      collectionMintAddress: nft.collection.address,
      collectionAuthority: authority,
      payer: keypair,
    })
    .run();

  await connection.confirmTransaction({
    signature: verifyResult.response.signature,
    ...(await connection.getLatestBlockhash()),
  });

  // Transfer nft to provided keypair
  const sendResult = await metaplex
    .nfts()
    .send({
      mintAddress: nft.mint.address,
      toOwner: keypair.publicKey,
    })
    .run();

  await connection.confirmTransaction({
    signature: sendResult.response.signature,
    ...(await connection.getLatestBlockhash()),
  });

  return { nft, collection };
}
export type LoanBorrower = Awaited<ReturnType<typeof askLoan>>;
export type LoanLender = Awaited<ReturnType<typeof giveLoan>>;

export async function askLoan(
  connection: anchor.web3.Connection,
  options: {
    amount: number;
    basisPoints: number;
    duration: number;
  }
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const { nft, collection } = await mintNFT(connection, keypair);

  const loanAddress = await findLoanAddress(
    nft.mint.address,
    keypair.publicKey
  );
  const collectionAddress = await findCollectionAddress(
    collection.mint.address
  );
  const tokenManager = await findTokenManagerAddress(
    nft.mint.address,
    keypair.publicKey
  );

  const largestAccounts = await connection.getTokenLargestAccounts(
    nft.mint.address
  );
  const depositTokenAccount = largestAccounts.value[0].address;

  const amount = new anchor.BN(options.amount);
  const basisPoints = new anchor.BN(options.basisPoints);
  const duration = new anchor.BN(options.duration);

  try {
    await program.methods
      .askLoan(amount, basisPoints, duration)
      .accounts({
        signer: signer.publicKey,
        tokenManager,
        depositTokenAccount,
        loan: loanAddress,
        collection: collectionAddress,
        mint: nft.mint.address,
        borrower: keypair.publicKey,
        edition: nft.edition.address,
        metadata: nft.metadataAddress,
        metadataProgram: METADATA_PROGRAM_ID,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([signer])
      .rpc();
  } catch (error) {
    console.log(error.logs);
    throw error;
  }

  return {
    keypair,
    provider,
    program,
    tokenManager,
    depositTokenAccount,
    loan: loanAddress,
    collection: collectionAddress,
    metadata: nft.metadataAddress,
    edition: nft.edition.address,
    mint: nft.mint.address,
  };
}

export async function giveLoan(
  connection: anchor.web3.Connection,
  borrower: Awaited<ReturnType<typeof askLoan>>
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  try {
    await program.methods
      .giveLoan()
      .accounts({
        signer: signer.publicKey,
        tokenManager: borrower.tokenManager,
        loan: borrower.loan,
        borrower: borrower.keypair.publicKey,
        lender: keypair.publicKey,
        mint: borrower.mint,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .signers([signer])
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

export type LoanOfferLender = Awaited<ReturnType<typeof offerLoan>>;
export type LoanOfferBorrower = Awaited<ReturnType<typeof takeLoan>>;

export async function offerLoan(
  connection: anchor.web3.Connection,
  options: {
    amount: number;
    basisPoints: number;
    duration: number;
  }
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const { nft, collection } = await mintNFT(connection, keypair);

  const amount = new anchor.BN(options.amount);
  const basisPoints = options.basisPoints;
  const duration = new anchor.BN(options.duration);
  const id = 0;

  const loanOffer = await findLoanOfferAddress(
    collection.address,
    keypair.publicKey,
    id
  );
  const escrowPaymentAccount = await findLoanOfferVaultAddress(loanOffer);
  const collectionAddress = await findCollectionAddress(
    collection.mint.address
  );

  await program.methods
    .offerLoan(amount, basisPoints, duration, id)
    .accounts({
      loanOffer,
      escrowPaymentAccount,
      collection: collectionAddress,
      signer: signer.publicKey,
      lender: keypair.publicKey,
    })
    .signers([signer])
    .rpc();

  return {
    keypair,
    provider,
    program,
    id,
    loanOffer,
    nft,
    escrowPaymentAccount,
    collection: collectionAddress,
  };
}

export async function takeLoan(
  connection: anchor.web3.Connection,
  lender: LoanOfferLender
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  // Transfer NFT from authority to borrower
  const authority = await getAuthority();
  const metaplex = await Metaplex.make(connection).use(
    keypairIdentity(authority)
  );

  await metaplex
    .nfts()
    .send({
      mintAddress: lender.nft.mint.address,
      toOwner: keypair.publicKey,
    })
    .run();

  const depositTokenAccount = (
    await connection.getTokenLargestAccounts(lender.nft.mint.address)
  ).value[0].address;

  const loanAddress = await findLoanAddress(
    lender.nft.mint.address,
    keypair.publicKey
  );
  const tokenManager = await findTokenManagerAddress(
    lender.nft.mint.address,
    keypair.publicKey
  );

  try {
    await program.methods
      .takeLoanOffer(new anchor.BN(0))
      .accounts({
        signer: signer.publicKey,
        tokenManager,
        depositTokenAccount,
        loan: loanAddress,
        loanOffer: lender.loanOffer,
        collection: lender.collection,
        escrowPaymentAccount: lender.escrowPaymentAccount,
        lender: lender.keypair.publicKey,
        borrower: keypair.publicKey,
        mint: lender.nft.mint.address,
        metadata: lender.nft.metadataAddress,
        edition: lender.nft.edition.address,
        metadataProgram: METADATA_PROGRAM_ID,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .signers([signer])
      .rpc();
  } catch (err) {
    console.log(err);
    console.log(err.logs);
    throw err;
  }

  return {
    keypair,
    provider,
    program,
    loan: loanAddress,
    loanOffer: lender.loanOffer,
    collection: lender.collection,
    escrowPaymentAccount: lender.escrowPaymentAccount,
  };
}

export type CallOptionBidBuyer = Awaited<ReturnType<typeof bidCallOption>>;
export type CallOptionBidSeller = Awaited<ReturnType<typeof sellCallOption>>;

export async function bidCallOption(
  connection: anchor.web3.Connection,
  options: {
    amount: number;
    strikePrice: number;
    expiry: number;
  }
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const { nft, collection } = await mintNFT(connection, keypair);

  const amount = new anchor.BN(options.amount);
  const strikePrice = new anchor.BN(options.strikePrice);
  const duration = new anchor.BN(options.expiry);
  const id = 0;

  const callOptionBid = await findCallOptionBidAddress(
    collection.address,
    keypair.publicKey,
    id
  );
  const escrowPaymentAccount = await findCallOptionBidVaultAddress(
    callOptionBid
  );
  const collectionAddress = await findCollectionAddress(
    collection.mint.address
  );

  await program.methods
    .bidCallOption(amount, strikePrice, duration, id)
    .accounts({
      callOptionBid,
      escrowPaymentAccount,
      collection: collectionAddress,
      signer: signer.publicKey,
      buyer: keypair.publicKey,
    })
    .signers([signer])
    .rpc();

  return {
    nft,
    keypair,
    provider,
    program,
    id,
    callOptionBid,
    escrowPaymentAccount,
    collection: collectionAddress,
  };
}

export async function sellCallOption(
  connection: anchor.web3.Connection,
  buyer: CallOptionBidBuyer
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  // Transfer NFT from authority to borrower
  const authority = await getAuthority();
  const metaplex = await Metaplex.make(connection).use(
    keypairIdentity(authority)
  );

  await metaplex
    .nfts()
    .send({
      mintAddress: buyer.nft.mint.address,
      toOwner: keypair.publicKey,
    })
    .run();

  const depositTokenAccount = (
    await connection.getTokenLargestAccounts(buyer.nft.mint.address)
  ).value[0].address;

  const callOptionAddress = await findCallOptionAddress(
    buyer.nft.mint.address,
    keypair.publicKey
  );
  const tokenManager = await findTokenManagerAddress(
    buyer.nft.mint.address,
    keypair.publicKey
  );

  try {
    await program.methods
      .sellCallOption(new anchor.BN(0))
      .accounts({
        signer: signer.publicKey,
        tokenManager,
        depositTokenAccount,
        callOption: callOptionAddress,
        callOptionBid: buyer.callOptionBid,
        collection: buyer.collection,
        escrowPaymentAccount: buyer.escrowPaymentAccount,
        buyer: buyer.keypair.publicKey,
        seller: keypair.publicKey,
        mint: buyer.nft.mint.address,
        metadata: buyer.nft.metadataAddress,
        edition: buyer.nft.edition.address,
        metadataProgram: METADATA_PROGRAM_ID,
      })
      .signers([signer])
      .rpc();
  } catch (err) {
    console.log(err);
    console.log(err.logs);
    throw err;
  }

  return {
    keypair,
    provider,
    program,
    callOption: callOptionAddress,
    callOptionBid: buyer.callOptionBid,
    collection: buyer.collection,
    escrowPaymentAccount: buyer.escrowPaymentAccount,
  };
}

export type CallOptionSeller = Awaited<ReturnType<typeof askCallOption>>;
export type CallOptionBuyer = Awaited<ReturnType<typeof buyCallOption>>;

export async function askCallOption(
  connection: anchor.web3.Connection,
  options: {
    amount: number;
    strikePrice: number;
    expiry: number;
  }
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);
  const { nft, collection } = await mintNFT(connection, keypair);
  const largestAccounts = await connection.getTokenLargestAccounts(
    nft.mint.address
  );
  const depositTokenAccount = largestAccounts.value[0].address;
  const callOptionAddress = await findCallOptionAddress(
    nft.mint.address,
    keypair.publicKey
  );
  const collectionAddress = await findCollectionAddress(
    collection.mint.address
  );
  const tokenManager = await findTokenManagerAddress(
    nft.mint.address,
    keypair.publicKey
  );

  const amount = new anchor.BN(options.amount);
  const strikePrice = new anchor.BN(options.strikePrice);
  const expiry = new anchor.BN(options.expiry);

  try {
    await program.methods
      .askCallOption(amount, strikePrice, expiry)
      .accounts({
        tokenManager,
        signer: signer.publicKey,
        callOption: callOptionAddress,
        collection: collectionAddress,
        mint: nft.mint.address,
        metadata: nft.metadataAddress,
        edition: nft.edition.address,
        seller: keypair.publicKey,
        depositTokenAccount: depositTokenAccount,
        metadataProgram: METADATA_PROGRAM_ID,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .signers([signer])
      .rpc();
  } catch (error) {
    console.log(error.logs);
    throw error;
  }

  return {
    keypair,
    provider,
    program,
    tokenManager,
    callOption: callOptionAddress,
    collection: collectionAddress,
    depositTokenAccount,
    mint: nft.mint.address,
    metatdata: nft.metadataAddress,
    edition: nft.edition.address,
  };
}

export async function buyCallOption(
  connection: anchor.web3.Connection,
  seller: Awaited<ReturnType<typeof askCallOption>>
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const metadata = await Metadata.fromAccountAddress(
    connection,
    seller.metatdata
  );

  const accounts = {
    signer: signer.publicKey,
    seller: seller.keypair.publicKey,
    buyer: keypair.publicKey,
    callOption: seller.callOption,
    tokenManager: seller.tokenManager,
    mint: seller.mint,
    metadata: seller.metatdata,
    edition: seller.edition,
    collection: seller.collection,
    metadataProgram: METADATA_PROGRAM_ID,
    systemProgram: anchor.web3.SystemProgram.programId,
    tokenProgram: splToken.TOKEN_PROGRAM_ID,
    clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
  };

  try {
    const signature = await program.methods
      .buyCallOption()
      .accounts(accounts)
      .remainingAccounts(
        metadata.data.creators.map((creator) => ({
          pubkey: creator.address,
          isSigner: false,
          isWritable: true,
        }))
      )
      .signers([signer])
      .rpc();

    const latestBlockhash = await connection.getLatestBlockhash();
    await connection.confirmTransaction({
      signature,
      ...latestBlockhash,
    });
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

export type RentalLender = Awaited<ReturnType<typeof initRental>>;
export type RentalBorrower = Awaited<ReturnType<typeof takeRental>>;

export async function initRental(
  connection: anchor.web3.Connection,
  options: {
    amount: number;
    expiry: number;
    borrower?: anchor.web3.PublicKey;
  }
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const { nft, collection } = await mintNFT(connection, keypair);

  const largestAccounts = await connection.getTokenLargestAccounts(
    nft.mint.address
  );
  const depositTokenAccount = largestAccounts.value[0].address;

  const rental = await findRentalAddress(nft.mint.address, keypair.publicKey);
  const rentalEscrow = await findRentalEscrowAddress(
    nft.mint.address,
    keypair.publicKey
  );
  const collectionAddress = await findCollectionAddress(
    collection.mint.address
  );
  const tokenManager = await findTokenManagerAddress(
    nft.mint.address,
    keypair.publicKey
  );

  const amount = new anchor.BN(options.amount);
  const expiry = new anchor.BN(options.expiry);
  const borrower = options.borrower ?? null;

  try {
    await program.methods
      .initRental({ amount, expiry, borrower })
      .accounts({
        rental,
        tokenManager,
        signer: signer.publicKey,
        collection: collectionAddress,
        lender: keypair.publicKey,
        depositTokenAccount: depositTokenAccount,
        metadata: nft.metadataAddress,
        mint: nft.mint.address,
        edition: nft.edition.address,
        metadataProgram: METADATA_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .signers([signer])
      .rpc();
  } catch (error) {
    console.log(error.logs);
    throw error;
  }

  return {
    keypair,
    program,
    provider,
    tokenManager,
    rental,
    rentalEscrow,
    collection: collectionAddress,
    depositTokenAccount,
    mint: nft.mint.address,
    edition: nft.edition.address,
    metadata: nft.metadataAddress,
  };
}

export async function takeRental(
  connection: anchor.web3.Connection,
  lender: Awaited<ReturnType<typeof initRental>>,
  days: number
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
    connection,
    keypair,
    lender.mint,
    keypair.publicKey
  );

  const metadataAccountInfo = await connection.getAccountInfo(lender.metadata);
  const [metadata] = Metadata.fromAccountInfo(metadataAccountInfo);

  try {
    await program.methods
      .takeRental(days)
      .accounts({
        signer: signer.publicKey,
        borrower: keypair.publicKey,
        lender: lender.keypair.publicKey,
        rental: lender.rental,
        rentalEscrow: lender.rentalEscrow,
        tokenManager: lender.tokenManager,
        depositTokenAccount: lender.depositTokenAccount,
        rentalTokenAccount: tokenAccount.address,
        mint: lender.mint,
        edition: lender.edition,
        metadata: lender.metadata,
        metadataProgram: METADATA_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .remainingAccounts(
        metadata.data.creators.map((creator) => ({
          pubkey: creator.address,
          isSigner: false,
          isWritable: true,
        }))
      )
      .signers([signer])
      .rpc();
  } catch (err) {
    console.log(err.logs);
    throw err;
  }

  return {
    keypair,
    provider,
    program,
    rentalTokenAccount: tokenAccount.address,
  };
}

export async function recoverRental(
  lender: RentalLender,
  borrower: RentalBorrower
) {
  const signer = await getSigner();

  try {
    await lender.program.methods
      .recoverRental()
      .accounts({
        signer: signer.publicKey,
        borrower: borrower.keypair.publicKey,
        lender: lender.keypair.publicKey,
        rental: lender.rental,
        rentalEscrow: lender.rentalEscrow,
        tokenManager: lender.tokenManager,
        depositTokenAccount: lender.depositTokenAccount,
        rentalTokenAccount: borrower.rentalTokenAccount,
        mint: lender.mint,
        edition: lender.edition,
        metadataProgram: METADATA_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
      })
      .signers([signer])
      .rpc();
  } catch (err) {
    console.log(err.logs);
    throw err;
  }
}

export async function wait(seconds) {
  await new Promise((resolve) => setTimeout(resolve, seconds * 1000));
}
