import * as anchor from "@project-serum/anchor";
import * as splToken from "@solana/spl-token";
import * as bip39 from "bip39";
import { derivePath } from "ed25519-hd-key";
import { Metaplex, keypairIdentity, Token } from "@metaplex-foundation/js";
import {
  createVerifyInstruction,
  Metadata,
  TokenStandard,
  VerificationArgs,
  PROGRAM_ID as METADATA_PROGRAM_ID,
} from "@metaplex-foundation/mpl-token-metadata";
import { PROGRAM_ID as AUTHORIZATION_RULES_PROGRAM_ID } from "@metaplex-foundation/mpl-token-auth-rules";
import { IDL, OndaListings } from "../target/types/onda_listings";

const PROGRAM_ID = new anchor.web3.PublicKey(
  "F2BTn5cmYkTzo52teXhG6jyLS3y2BujdE56yZaGyvxwC"
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
): anchor.Program<OndaListings> {
  return new anchor.Program<OndaListings>(IDL, PROGRAM_ID, provider);
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

export function findCollectionAddress(
  collection: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [collectionAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("collection"), collection.toBuffer()],
    PROGRAM_ID
  );

  return collectionAddress;
}

export function findTokenManagerAddress(
  mint: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [tokenManagerAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("token_manager"), mint.toBuffer()],
    PROGRAM_ID
  );

  return tokenManagerAddress;
}

export function findLoanAddress(
  mint: anchor.web3.PublicKey,
  borrower: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [loanAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("loan"), mint.toBuffer(), borrower.toBuffer()],
    PROGRAM_ID
  );

  return loanAddress;
}

export function findLoanOfferAddress(
  collectionMint: anchor.web3.PublicKey,
  lender: anchor.web3.PublicKey,
  id: number
): anchor.web3.PublicKey {
  const [loanOfferAddress] = anchor.web3.PublicKey.findProgramAddressSync(
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

export function findLoanOfferVaultAddress(
  loanOffer: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [loanOfferVaultAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("loan_offer_vault"), loanOffer.toBuffer()],
    PROGRAM_ID
  );

  return loanOfferVaultAddress;
}

export function findCallOptionBidAddress(
  collectionMint: anchor.web3.PublicKey,
  buyer: anchor.web3.PublicKey,
  id: number
): anchor.web3.PublicKey {
  const [callOptionBidAddress] = anchor.web3.PublicKey.findProgramAddressSync(
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

export function findCallOptionBidVaultAddress(
  callOptionBid: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [callOptionBidAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("call_option_bid_vault"), callOptionBid.toBuffer()],
    PROGRAM_ID
  );

  return callOptionBidAddress;
}

export function findCallOptionAddress(
  mint: anchor.web3.PublicKey,
  seller: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [callOptionAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("call_option"), mint.toBuffer(), seller.toBuffer()],
    PROGRAM_ID
  );

  return callOptionAddress;
}

export function findRentalAddress(
  mint: anchor.web3.PublicKey,
  lender: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [rentalAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rental"), mint.toBuffer(), lender.toBuffer()],
    PROGRAM_ID
  );

  return rentalAddress;
}

export function findRentalEscrowAddress(
  mint: anchor.web3.PublicKey,
  lender: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [rentalEscrowAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rental_escrow"), mint.toBuffer(), lender.toBuffer()],
    PROGRAM_ID
  );

  return rentalEscrowAddress;
}

export function findMetadataAddress(
  mint: anchor.web3.PublicKey
): anchor.web3.PublicKey {
  const [metadataAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("metadata"), METADATA_PROGRAM_ID.toBuffer(), mint.toBuffer()],
    METADATA_PROGRAM_ID
  );

  return metadataAddress;
}

export function findEscrowTokenAccount(tokenManager: anchor.web3.PublicKey) {
  const [escrowTokenAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("escrow"), tokenManager.toBuffer()],
    PROGRAM_ID
  );

  return escrowTokenAccount;
}

export function findTokenRecordAddress(
  mint: anchor.web3.PublicKey,
  tokenAccount: anchor.web3.PublicKey
) {
  const [tokenRecordAddress] = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("metadata"),
      METADATA_PROGRAM_ID.toBuffer(),
      mint.toBuffer(),
      Buffer.from("token_record"),
      tokenAccount.toBuffer(),
    ],
    METADATA_PROGRAM_ID
  );

  return tokenRecordAddress;
}

export async function mintNFT(
  connection: anchor.web3.Connection,
  keypair: anchor.web3.Keypair,
  tokenStandard: TokenStandard = TokenStandard.ProgrammableNonFungible
) {
  const authority = await getAuthority();
  const signer = await getSigner();
  const provider = getProvider(connection, authority);
  const program = getProgram(provider);
  await requestAirdrop(connection, authority.publicKey);

  const metaplex = Metaplex.make(connection).use(keypairIdentity(authority));

  const { nft: collection } = await metaplex.nfts().create({
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
    tokenStandard: TokenStandard.ProgrammableNonFungible,
  });

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

  const { nft } = await metaplex.nfts().create({
    tokenStandard,
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
  });

  let latestBlockhash = await connection.getLatestBlockhash();
  const verifyIx = createVerifyInstruction(
    {
      authority: authority.publicKey,
      metadata: nft.metadataAddress,
      collectionMint: nft.collection.address,
      collectionMetadata: collection.metadataAddress,
      collectionMasterEdition: collection.edition.address,
      systemProgram: anchor.web3.SystemProgram.programId,
      sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
    },
    {
      verificationArgs: VerificationArgs.CollectionV1,
    }
  );
  const messageV0 = new anchor.web3.TransactionMessage({
    payerKey: authority.publicKey,
    recentBlockhash: latestBlockhash.blockhash,
    instructions: [verifyIx],
  }).compileToV0Message();

  const transaction = new anchor.web3.VersionedTransaction(messageV0);
  transaction.sign([authority]);

  const verifySignature = await connection.sendTransaction(transaction);
  await connection.confirmTransaction({
    signature: verifySignature,
    ...latestBlockhash,
  });

  // Transfer nft to provided keypair
  if (!keypair.publicKey.equals(authority.publicKey)) {
    const sendResult = await metaplex.nfts().transfer({
      nftOrSft: nft,
      toOwner: keypair.publicKey,
    });

    await connection.confirmTransaction({
      signature: sendResult.response.signature,
      ...(await connection.getLatestBlockhash()),
    });
  }

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
  const tokenManager = await findTokenManagerAddress(nft.mint.address);

  const largestAccounts = await connection.getTokenLargestAccounts(
    nft.mint.address
  );
  const depositTokenAccount = largestAccounts.value[0].address;

  const amount = new anchor.BN(options.amount);
  const basisPoints = options.basisPoints;
  const duration = new anchor.BN(options.duration);

  const accounts = {
    signer: signer.publicKey,
    tokenManager,
    depositTokenAccount,
    tokenRecord: null,
    loan: loanAddress,
    collection: collectionAddress,
    mint: nft.mint.address,
    borrower: keypair.publicKey,
    edition: nft.edition.address,
    metadata: nft.metadataAddress,
    metadataProgram: METADATA_PROGRAM_ID,
    authorizationRules: null,
    authorizationRulesProgram: AUTHORIZATION_RULES_PROGRAM_ID,
    tokenProgram: splToken.TOKEN_PROGRAM_ID,
    rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    systemProgram: anchor.web3.SystemProgram.programId,
    sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
  };

  if (nft.tokenStandard === TokenStandard.ProgrammableNonFungible) {
    accounts.tokenRecord = findTokenRecordAddress(
      nft.mint.address,
      depositTokenAccount
    );
  }

  try {
    await program.methods
      .askLoan(amount, basisPoints, duration)
      .accounts(accounts)
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
    tokenRecord: accounts.tokenRecord,
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
    tokenStandard?: TokenStandard;
  }
) {
  const keypair = anchor.web3.Keypair.generate();
  const signer = await getSigner();
  const provider = getProvider(connection, keypair);
  const program = getProgram(provider);
  await requestAirdrop(connection, keypair.publicKey);

  const { nft, collection } = await mintNFT(
    connection,
    keypair,
    options.tokenStandard
  );

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

  try {
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
  } catch (err) {
    console.log(err.logs);
    throw err;
  }

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

  // Transfer NFT from lender to borrower
  const metaplex = await Metaplex.make(connection).use(
    keypairIdentity(lender.keypair)
  );
  await metaplex.nfts().transfer({
    nftOrSft: lender.nft,
    toOwner: keypair.publicKey,
  });

  const depositTokenAccount = (
    await connection.getTokenLargestAccounts(lender.nft.mint.address)
  ).value[0].address;

  const loanAddress = await findLoanAddress(
    lender.nft.mint.address,
    keypair.publicKey
  );
  const tokenManager = await findTokenManagerAddress(lender.nft.mint.address);

  const accounts = {
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
    tokenRecord: null,
    metadataProgram: METADATA_PROGRAM_ID,
    authorizationRules: null,
    authorizationRulesProgram: AUTHORIZATION_RULES_PROGRAM_ID,
    tokenProgram: splToken.TOKEN_PROGRAM_ID,
    clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
    sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
  };

  if (lender.nft.tokenStandard === TokenStandard.ProgrammableNonFungible) {
    accounts.tokenRecord = await findTokenRecordAddress(
      lender.nft.mint.address,
      depositTokenAccount
    );
  }

  try {
    await program.methods
      .takeLoanOffer(0)
      .accounts(accounts)
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
    depositTokenAccount,
    tokenManager,
    tokenRecord: accounts.tokenRecord,
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

  await metaplex.nfts().transfer({
    nftOrSft: buyer.nft,
    toOwner: keypair.publicKey,
  });

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
      .sellCallOption(0)
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
