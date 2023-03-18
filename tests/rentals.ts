require("dotenv").config();

import assert from "assert";
import {
  Metadata,
  PROGRAM_ID as METADATA_PROGRAM_ID,
} from "@metaplex-foundation/mpl-token-metadata";
import * as anchor from "@project-serum/anchor";
import * as splToken from "@solana/spl-token";
import * as helpers from "./helpers";

// Configure the client to use the local cluster.
const connection = new anchor.web3.Connection(
  "http://127.0.0.1:8899",
  anchor.AnchorProvider.defaultOptions().preflightCommitment
);

// describe("Rentals", () => {
//   describe("Specified borrower", async () => {
//     let lender: helpers.RentalLender;
//     let borrowerTokenAccount: anchor.web3.PublicKey;
//     let options;
//     let privateBorrower = anchor.web3.Keypair.generate();

//     it("Initializes a rental with a borrower", async () => {
//       options = {
//         amount: 0,
//         expiry: Date.now() / 1000 + 84_600 * 3,
//         borrower: privateBorrower.publicKey,
//       };
//       lender = await helpers.initRental(connection, options);

//       const rental = await lender.program.account.rental.fetch(lender.rental);
//       const tokenAddress = (
//         await connection.getTokenLargestAccounts(lender.mint)
//       ).value[0].address;
//       const tokenAccount = await splToken.getAccount(connection, tokenAddress);

//       assert.ok(tokenAccount.isFrozen);
//       assert.ok(
//         tokenAccount.delegate.toBase58(),
//         lender.tokenManager.toBase58()
//       );
//       assert.equal(rental.amount.toNumber(), options.amount);
//       assert.equal(
//         rental.lender.toBase58(),
//         lender.keypair.publicKey.toBase58()
//       );
//       assert.equal(rental.borrower.toBase58(), options.borrower.toBase58());
//       assert.deepEqual(rental.state, { listed: {} });
//     });

//     it("Does not allow a different address to take the rental", async () => {
//       const signer = await helpers.getSigner();
//       const newKeypair = anchor.web3.Keypair.generate();
//       await helpers.requestAirdrop(connection, newKeypair.publicKey);
//       const provider = helpers.getProvider(connection, newKeypair);
//       const program = helpers.getProgram(provider);

//       const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
//         connection,
//         newKeypair,
//         lender.mint,
//         newKeypair.publicKey
//       );
//       const metadata = await Metadata.fromAccountAddress(
//         connection,
//         lender.metadata
//       );

//       try {
//         await program.methods
//           .takeRental(1)
//           .accounts({
//             signer: signer.publicKey,
//             borrower: newKeypair.publicKey,
//             lender: lender.keypair.publicKey,
//             rental: lender.rental,
//             rentalEscrow: lender.rentalEscrow,
//             tokenManager: lender.tokenManager,
//             depositTokenAccount: lender.depositTokenAccount,
//             rentalTokenAccount: tokenAccount.address,
//             mint: lender.mint,
//             edition: lender.edition,
//             metadata: lender.metadata,
//             metadataProgram: METADATA_PROGRAM_ID,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: splToken.TOKEN_PROGRAM_ID,
//             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//           })
//           .remainingAccounts(
//             metadata.data.creators.map((creator) => ({
//               pubkey: creator.address,
//               isSigner: false,
//               isWritable: true,
//             }))
//           )
//           .signers([signer])
//           .rpc();
//         assert.fail("Expected to fail");
//       } catch (err) {
//         assert(err instanceof anchor.AnchorError);
//         assert.equal(err.error.errorCode.number, 2502);
//         assert.equal(err.error.errorCode.code, "RequireKeysEqViolated");
//       }
//     });

//     it("Allows a rental to be taken by the borrower", async () => {
//       const signer = await helpers.getSigner();
//       await helpers.requestAirdrop(connection, privateBorrower.publicKey);
//       const provider = helpers.getProvider(connection, privateBorrower);
//       const program = helpers.getProgram(provider);

//       borrowerTokenAccount = await splToken.createAccount(
//         connection,
//         privateBorrower,
//         lender.mint,
//         privateBorrower.publicKey
//       );
//       const metadata = await Metadata.fromAccountAddress(
//         connection,
//         lender.metadata
//       );

//       const days = 1;
//       const estimatedCurrentExpiry = Math.round(
//         Date.now() / 1000 + 86_400 * days
//       );

//       await program.methods
//         .takeRental(days)
//         .accounts({
//           signer: signer.publicKey,
//           borrower: privateBorrower.publicKey,
//           lender: lender.keypair.publicKey,
//           rental: lender.rental,
//           rentalEscrow: lender.rentalEscrow,
//           tokenManager: lender.tokenManager,
//           depositTokenAccount: lender.depositTokenAccount,
//           rentalTokenAccount: borrowerTokenAccount,
//           mint: lender.mint,
//           edition: lender.edition,
//           metadata: lender.metadata,
//           metadataProgram: METADATA_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           tokenProgram: splToken.TOKEN_PROGRAM_ID,
//           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//         })
//         .remainingAccounts(
//           metadata.data.creators.map((creator) => ({
//             pubkey: creator.address,
//             isSigner: false,
//             isWritable: true,
//           }))
//         )
//         .signers([signer])
//         .rpc();

//       const rental = await lender.program.account.rental.fetch(lender.rental);
//       const tokenAccount = await splToken.getAccount(
//         connection,
//         borrowerTokenAccount
//       );

//       assert.deepEqual(rental.state, { rented: {} });
//       assert.equal(tokenAccount.isFrozen, true, "isFrozen");
//       assert.equal(tokenAccount.amount, BigInt(1));
//       assert.equal(
//         rental.borrower.toBase58(),
//         privateBorrower.publicKey.toBase58(),
//         "borrower"
//       );
//       assert.ok(
//         rental.currentExpiry.toNumber() >= estimatedCurrentExpiry - 2 &&
//           rental.currentExpiry.toNumber() <= estimatedCurrentExpiry + 2,
//         "currentExpiry"
//       );
//     });

//     it("Does not allow rental token account to be closed", async () => {
//       try {
//         await splToken.closeAccount(
//           connection,
//           privateBorrower,
//           borrowerTokenAccount,
//           lender.depositTokenAccount,
//           privateBorrower
//         );
//         assert.fail();
//       } catch (err) {
//         assert.ok(
//           err.logs.includes(
//             "Program log: Error: Non-native account can only be closed if its balance is zero"
//           )
//         );
//       }
//     });

//     it("Does not allow a rental to be recovered before expiry", async () => {
//       const signer = await helpers.getSigner();
//       const metadata = await Metadata.fromAccountAddress(
//         connection,
//         lender.metadata
//       );

//       try {
//         await lender.program.methods
//           .recoverRental()
//           .accounts({
//             signer: signer.publicKey,
//             borrower: privateBorrower.publicKey,
//             lender: lender.keypair.publicKey,
//             rental: lender.rental,
//             rentalEscrow: lender.rentalEscrow,
//             tokenManager: lender.tokenManager,
//             depositTokenAccount: lender.depositTokenAccount,
//             rentalTokenAccount: borrowerTokenAccount,
//             mint: lender.mint,
//             metadata: lender.metadata,
//             edition: lender.edition,
//             metadataProgram: METADATA_PROGRAM_ID,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: splToken.TOKEN_PROGRAM_ID,
//             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//           })
//           .remainingAccounts(
//             metadata.data.creators.map((creator) => ({
//               pubkey: creator.address,
//               isSigner: false,
//               isWritable: true,
//             }))
//           )
//           .signers([signer])
//           .rpc();
//         assert.fail();
//       } catch (err) {
//         assert(err instanceof anchor.AnchorError);
//         assert.equal(err.error.errorCode.number, 6001);
//         assert.equal(err.error.errorCode.code, "NotExpired");
//       }
//     });
//   });

//   describe("Open rental", async () => {
//     let options;
//     let lender: helpers.RentalLender;
//     let borrower: helpers.RentalBorrower;

//     it("Initializes an open rental", async () => {
//       options = {
//         amount: 10_000,
//         expiry: Date.now() / 1000 + 86_400 * 180,
//       };
//       lender = await helpers.initRental(connection, options);

//       const rental = await lender.program.account.rental.fetch(lender.rental);
//       const tokenAddress = (
//         await connection.getTokenLargestAccounts(lender.mint)
//       ).value[0].address;
//       const tokenAccount = await splToken.getAccount(connection, tokenAddress);

//       assert.ok(tokenAccount.isFrozen);
//       assert.ok(
//         tokenAccount.delegate.toBase58(),
//         lender.tokenManager.toBase58()
//       );
//       assert.equal(rental.amount.toNumber(), options.amount);
//       assert.equal(
//         rental.lender.toBase58(),
//         lender.keypair.publicKey.toBase58()
//       );
//       assert.equal(rental.borrower, null);
//       assert.deepEqual(rental.state, { listed: {} });
//     });

//     it("Allows a rental to be taken for x days", async () => {
//       const days = 2;
//       const estimatedCurrentExpiry = Math.round(
//         Date.now() / 1000 + 86_400 * days
//       );
//       borrower = await helpers.takeRental(connection, lender, days);

//       const rental = await lender.program.account.rental.fetch(lender.rental);
//       const tokenAddress = (
//         await connection.getTokenLargestAccounts(lender.mint)
//       ).value[0].address;

//       const tokenAccount = await splToken.getAccount(connection, tokenAddress);

//       assert.deepEqual(rental.state, { rented: {} });
//       assert.equal(tokenAccount.isFrozen, true);
//       assert.equal(tokenAccount.amount, BigInt(1));
//       assert.equal(
//         rental.borrower.toBase58(),
//         borrower.keypair.publicKey.toBase58()
//       );
//       assert.ok(
//         rental.currentExpiry.toNumber() >= estimatedCurrentExpiry - 2 &&
//           rental.currentExpiry.toNumber() <= estimatedCurrentExpiry + 2
//       );
//     });
//   });

//   describe("Loan repayment with active rental", () => {
//     let borrower: helpers.LoanBorrower;
//     let lender: helpers.LoanLender;
//     let rentalTokenAccount: anchor.web3.PublicKey;
//     let thirdPartyKeypair = anchor.web3.Keypair.generate();
//     let options = {
//       amount: anchor.web3.LAMPORTS_PER_SOL,
//       basisPoints: 1000,
//       duration: 86_400 * 365, // 1 year
//     };

//     it("Allows collateralized NFTs to be listed for rental", async () => {
//       const signer = await helpers.getSigner();
//       borrower = await helpers.askLoan(connection, options);
//       lender = await helpers.giveLoan(connection, borrower);

//       const amount = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL / 100);
//       const expiry = new anchor.BN(Date.now() / 1000 + 86_400 * 3);

//       const rentalAddress = await helpers.findRentalAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );

//       const tokenManagerAddress = await helpers.findTokenManagerAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );

//       await borrower.program.methods
//         .initRental({ amount, expiry, borrower: null })
//         .accounts({
//           signer: signer.publicKey,
//           rental: rentalAddress,
//           collection: borrower.collection,
//           tokenManager: tokenManagerAddress,
//           lender: borrower.keypair.publicKey,
//           depositTokenAccount: borrower.depositTokenAccount,
//           metadata: borrower.metadata,
//           mint: borrower.mint,
//           edition: borrower.edition,
//           metadataProgram: METADATA_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           tokenProgram: splToken.TOKEN_PROGRAM_ID,
//           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//         })
//         .signers([signer])
//         .rpc();

//       const rental = await lender.program.account.rental.fetch(rentalAddress);
//       const tokenManager = await lender.program.account.tokenManager.fetch(
//         tokenManagerAddress
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         loan: true,
//         rental: true,
//         callOption: false,
//       });
//       assert.equal(rental.borrower, null);
//       assert.deepEqual(rental.state, { listed: {} });
//     });

//     it("Allows collateralized NFTs to be rented", async () => {
//       const signer = await helpers.getSigner();
//       await helpers.requestAirdrop(connection, thirdPartyKeypair.publicKey);
//       const provider = helpers.getProvider(connection, thirdPartyKeypair);
//       const program = helpers.getProgram(provider);
//       const rentalAddress = await helpers.findRentalAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const rentalEscrowAddress = await helpers.findRentalEscrowAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const tokenManagerAddress = await helpers.findTokenManagerAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const rentalTokenAccount =
//         await splToken.getOrCreateAssociatedTokenAccount(
//           connection,
//           thirdPartyKeypair,
//           borrower.mint,
//           thirdPartyKeypair.publicKey
//         );
//       const [metadataAddress] = await helpers.findMetadataAddress(
//         borrower.mint
//       );

//       try {
//         await program.methods
//           .takeRental(2)
//           .accounts({
//             signer: signer.publicKey,
//             borrower: thirdPartyKeypair.publicKey,
//             lender: borrower.keypair.publicKey,
//             rental: rentalAddress,
//             rentalEscrow: rentalEscrowAddress,
//             tokenManager: tokenManagerAddress,
//             depositTokenAccount: borrower.depositTokenAccount,
//             rentalTokenAccount: rentalTokenAccount.address,
//             mint: borrower.mint,
//             edition: borrower.edition,
//             metadata: metadataAddress,
//             metadataProgram: METADATA_PROGRAM_ID,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: splToken.TOKEN_PROGRAM_ID,
//             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//           })
//           .signers([signer])
//           .rpc();
//       } catch (error) {
//         console.log(error.logs);
//         throw error;
//       }

//       const rental = await lender.program.account.rental.fetch(rentalAddress);
//       const tokenManager = await lender.program.account.tokenManager.fetch(
//         tokenManagerAddress
//       );
//       const tokenAccount = await splToken.getAccount(
//         connection,
//         rentalTokenAccount.address
//       );
//       assert.deepEqual(tokenManager.accounts, {
//         loan: true,
//         rental: true,
//         callOption: false,
//       });
//       assert.equal(
//         rental.borrower.toBase58(),
//         thirdPartyKeypair.publicKey.toBase58()
//       );
//       assert.deepEqual(rental.state, { rented: {} });
//       assert.equal(tokenAccount.amount, BigInt(1));
//       assert.ok(tokenAccount.isFrozen);
//       assert.ok(tokenAccount.delegate.equals(tokenManagerAddress));
//       assert.equal(tokenAccount.delegatedAmount, BigInt(1));
//     });

//     it("Allows loans to be repaid", async () => {
//       const signer = await helpers.getSigner();
//       const metadata = await Metadata.fromAccountAddress(
//         connection,
//         borrower.metadata
//       );
//       const lenderPreRepaymentBalance = await connection.getBalance(
//         lender.keypair.publicKey
//       );

//       const preTokenManager = await borrower.program.account.tokenManager.fetch(
//         borrower.tokenManager
//       );

//       await borrower.program.methods
//         .repayLoan()
//         .accounts({
//           signer: signer.publicKey,
//           loan: borrower.loan,
//           tokenManager: borrower.tokenManager,
//           borrower: borrower.keypair.publicKey,
//           depositTokenAccount: borrower.depositTokenAccount,
//           lender: lender.keypair.publicKey,
//           mint: borrower.mint,
//           metadata: borrower.metadata,
//           edition: borrower.edition,
//           metadataProgram: METADATA_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           tokenProgram: splToken.TOKEN_PROGRAM_ID,
//           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//         })
//         .remainingAccounts([
//           ...metadata.data.creators.map((creator) => ({
//             pubkey: creator.address,
//             isSigner: false,
//             isWritable: true,
//           })),
//         ])
//         .signers([signer])
//         .rpc();

//       const lenderPostRepaymentBalance = await connection.getBalance(
//         lender.keypair.publicKey
//       );
//       const borrowerTokenAccount = await splToken.getAccount(
//         connection,
//         borrower.depositTokenAccount
//       );
//       const rentalTokenAccount =
//         await splToken.getOrCreateAssociatedTokenAccount(
//           connection,
//           thirdPartyKeypair,
//           borrower.mint,
//           thirdPartyKeypair.publicKey
//         );
//       const tokenManager = await borrower.program.account.tokenManager.fetch(
//         borrower.tokenManager
//       );
//       assert.deepEqual(tokenManager.accounts, {
//         rental: true,
//         callOption: false,
//         loan: false,
//       });
//       assert.equal(
//         borrowerTokenAccount.amount,
//         BigInt(0),
//         "borrower token amount"
//       );
//       assert.equal(rentalTokenAccount.amount, BigInt(1), "rental token amount");
//       assert.ok(rentalTokenAccount.isFrozen, "rental token frozen");
//       assert.ok(
//         lenderPostRepaymentBalance > lenderPreRepaymentBalance,
//         "balance"
//       );
//     });
//   });

//   describe("Repossession with active rental", () => {
//     let borrower: helpers.LoanBorrower;
//     let lender: helpers.LoanLender;
//     let thirdPartyKeypair = anchor.web3.Keypair.generate();

//     it("Allows collateralized NFTs to be listed for rental", async () => {
//       const signer = await helpers.getSigner();
//       borrower = await helpers.askLoan(connection, {
//         amount: anchor.web3.LAMPORTS_PER_SOL / 100,
//         basisPoints: 500,
//         duration: 1,
//       });
//       lender = await helpers.giveLoan(connection, borrower);

//       const amount = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL / 100);
//       const expiry = new anchor.BN(Date.now() / 1000 + 86_400 * 3);

//       const rentalAddress = await helpers.findRentalAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );

//       const tokenManagerAddress = await helpers.findTokenManagerAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );

//       await borrower.program.methods
//         .initRental({ amount, expiry, borrower: null })
//         .accounts({
//           signer: signer.publicKey,
//           rental: rentalAddress,
//           collection: borrower.collection,
//           tokenManager: tokenManagerAddress,
//           lender: borrower.keypair.publicKey,
//           depositTokenAccount: borrower.depositTokenAccount,
//           metadata: borrower.metadata,
//           mint: borrower.mint,
//           edition: borrower.edition,
//           metadataProgram: METADATA_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           tokenProgram: splToken.TOKEN_PROGRAM_ID,
//           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//         })
//         .signers([signer])
//         .rpc();

//       const rental = await lender.program.account.rental.fetch(rentalAddress);
//       const tokenManager = await lender.program.account.tokenManager.fetch(
//         tokenManagerAddress
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         loan: true,
//         rental: true,
//         callOption: false,
//       });
//       assert.equal(rental.borrower, null);
//       assert.deepEqual(rental.state, { listed: {} });
//     });

//     it("Allows collateralized NFTs to be rented", async () => {
//       const signer = await helpers.getSigner();
//       await helpers.requestAirdrop(connection, thirdPartyKeypair.publicKey);
//       const provider = helpers.getProvider(connection, thirdPartyKeypair);
//       const program = helpers.getProgram(provider);
//       const rentalAddress = await helpers.findRentalAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const rentalEscrowAddress = await helpers.findRentalEscrowAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const tokenManagerAddress = await helpers.findTokenManagerAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const rentalTokenAccount =
//         await splToken.getOrCreateAssociatedTokenAccount(
//           connection,
//           thirdPartyKeypair,
//           borrower.mint,
//           thirdPartyKeypair.publicKey
//         );
//       const [metadataAddress] = await helpers.findMetadataAddress(
//         borrower.mint
//       );

//       try {
//         await program.methods
//           .takeRental(2)
//           .accounts({
//             signer: signer.publicKey,
//             borrower: thirdPartyKeypair.publicKey,
//             lender: borrower.keypair.publicKey,
//             rental: rentalAddress,
//             rentalEscrow: rentalEscrowAddress,
//             tokenManager: tokenManagerAddress,
//             depositTokenAccount: borrower.depositTokenAccount,
//             rentalTokenAccount: rentalTokenAccount.address,
//             mint: borrower.mint,
//             edition: borrower.edition,
//             metadata: metadataAddress,
//             metadataProgram: METADATA_PROGRAM_ID,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: splToken.TOKEN_PROGRAM_ID,
//             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//           })
//           .signers([signer])
//           .rpc();
//       } catch (error) {
//         console.log(error.logs);
//         throw error;
//       }

//       const rental = await lender.program.account.rental.fetch(rentalAddress);
//       const tokenManager = await lender.program.account.tokenManager.fetch(
//         tokenManagerAddress
//       );
//       const tokenAccount = await splToken.getAccount(
//         connection,
//         rentalTokenAccount.address
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         loan: true,
//         rental: true,
//         callOption: false,
//       });
//       assert.equal(
//         rental.borrower.toBase58(),
//         thirdPartyKeypair.publicKey.toBase58()
//       );
//       assert.deepEqual(rental.state, { rented: {} });
//       assert.equal(tokenAccount.amount, BigInt(1));
//       assert.ok(tokenAccount.isFrozen);
//       assert.ok(tokenAccount.delegate.equals(tokenManagerAddress));
//       assert.equal(tokenAccount.delegatedAmount, BigInt(1));
//     });

//     it("Will settle rental fees when collateral is repossessed", async () => {
//       const signer = await helpers.getSigner();
//       const metadata = await Metadata.fromAccountAddress(
//         connection,
//         borrower.metadata
//       );
//       await helpers.wait(10); // Wait to allow some rent to accrue

//       const rentalAddress = await helpers.findRentalAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const rentalEscrowAddress = await helpers.findRentalEscrowAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const rentalTokenAccount =
//         await splToken.getOrCreateAssociatedTokenAccount(
//           connection,
//           thirdPartyKeypair,
//           borrower.mint,
//           thirdPartyKeypair.publicKey
//         );
//       const lenderTokenAccount =
//         await splToken.getOrCreateAssociatedTokenAccount(
//           connection,
//           lender.keypair,
//           borrower.mint,
//           lender.keypair.publicKey
//         );
//       const renterBalanceBefore = await connection.getBalance(
//         thirdPartyKeypair.publicKey
//       );

//       try {
//         const accounts = {
//           signer: signer.publicKey,
//           rental: rentalAddress,
//           rentalEscrow: rentalEscrowAddress,
//           borrower: borrower.keypair.publicKey,
//           lender: lender.keypair.publicKey,
//           lenderTokenAccount: lenderTokenAccount.address,
//           tokenAccount: rentalTokenAccount.address,
//           loan: borrower.loan,
//           tokenManager: borrower.tokenManager,
//           mint: borrower.mint,
//           metadata: borrower.metadata,
//           edition: borrower.edition,
//           metadataProgram: METADATA_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           tokenProgram: splToken.TOKEN_PROGRAM_ID,
//           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//           rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//         };

//         await lender.program.methods
//           .repossessWithRental()
//           .accounts(accounts)
//           .remainingAccounts([
//             ...metadata.data.creators.map((creator) => ({
//               pubkey: creator.address,
//               isSigner: false,
//               isWritable: true,
//             })),
//             {
//               isSigner: false,
//               isWritable: true,
//               pubkey: thirdPartyKeypair.publicKey,
//             },
//           ])
//           .signers([signer])
//           .rpc();
//       } catch (err) {
//         console.log(err.logs);
//         throw err;
//       }

//       const updatedLendertokenAccount = await splToken.getAccount(
//         connection,
//         lenderTokenAccount.address
//       );
//       const updatedRentalTokenAccount = await splToken.getAccount(
//         connection,
//         rentalTokenAccount.address
//       );
//       const tokenManager = await borrower.program.account.tokenManager.fetch(
//         borrower.tokenManager
//       );
//       const defaultedLoan = await borrower.program.account.loan.fetch(
//         borrower.loan
//       );
//       const renterBalance = await connection.getBalance(
//         thirdPartyKeypair.publicKey
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         rental: false,
//         callOption: false,
//         loan: false,
//       });
//       assert.equal(updatedLendertokenAccount.amount, BigInt(1));
//       assert.equal(updatedRentalTokenAccount.amount, BigInt(0));
//       assert.deepEqual(defaultedLoan.state, { defaulted: {} });
//       assert.ok(renterBalance > renterBalanceBefore, "renter balance");
//     });
//   });

//   describe("Repossession with listed rental", () => {
//     let borrower: helpers.LoanBorrower;
//     let lender: helpers.LoanLender;

//     it("Allows collateralized NFTs to be listed for rental", async () => {
//       const signer = await helpers.getSigner();
//       borrower = await helpers.askLoan(connection, {
//         amount: anchor.web3.LAMPORTS_PER_SOL / 100,
//         basisPoints: 500,
//         duration: 1,
//       });
//       lender = await helpers.giveLoan(connection, borrower);

//       const amount = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL / 100);
//       const expiry = new anchor.BN(Date.now() / 1000 + 86_400 * 3);

//       const rentalAddress = await helpers.findRentalAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const tokenManagerAddress = await helpers.findTokenManagerAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       await borrower.program.methods
//         .initRental({ amount, expiry, borrower: null })
//         .accounts({
//           signer: signer.publicKey,
//           rental: rentalAddress,
//           collection: borrower.collection,
//           tokenManager: tokenManagerAddress,
//           lender: borrower.keypair.publicKey,
//           depositTokenAccount: borrower.depositTokenAccount,
//           mint: borrower.mint,
//           metadata: borrower.metadata,
//           edition: borrower.edition,
//           metadataProgram: METADATA_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           tokenProgram: splToken.TOKEN_PROGRAM_ID,
//           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//         })
//         .signers([signer])
//         .rpc();

//       const rental = await lender.program.account.rental.fetch(rentalAddress);
//       const tokenManager = await lender.program.account.tokenManager.fetch(
//         tokenManagerAddress
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         loan: true,
//         rental: true,
//         callOption: false,
//       });
//       assert.equal(rental.borrower, null);
//       assert.deepEqual(rental.state, { listed: {} });
//     });

//     it("Will settle rental fees when collateral is repossessed", async () => {
//       const signer = await helpers.getSigner();
//       const metadata = await Metadata.fromAccountAddress(
//         connection,
//         borrower.metadata
//       );
//       const rentalAddress = await helpers.findRentalAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const rentalEscrowAddress = await helpers.findRentalEscrowAddress(
//         borrower.mint,
//         borrower.keypair.publicKey
//       );
//       const lenderTokenAccount =
//         await splToken.getOrCreateAssociatedTokenAccount(
//           connection,
//           lender.keypair,
//           borrower.mint,
//           lender.keypair.publicKey
//         );

//       try {
//         await lender.program.methods
//           .repossessWithRental()
//           .accounts({
//             signer: signer.publicKey,
//             rental: rentalAddress,
//             rentalEscrow: rentalEscrowAddress,
//             borrower: borrower.keypair.publicKey,
//             lender: lender.keypair.publicKey,
//             lenderTokenAccount: lenderTokenAccount.address,
//             tokenAccount: borrower.depositTokenAccount,
//             loan: borrower.loan,
//             tokenManager: borrower.tokenManager,
//             mint: borrower.mint,
//             metadata: borrower.metadata,
//             edition: borrower.edition,
//             metadataProgram: METADATA_PROGRAM_ID,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: splToken.TOKEN_PROGRAM_ID,
//             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//             rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//           })
//           .remainingAccounts([
//             ...metadata.data.creators.map((creator) => ({
//               pubkey: creator.address,
//               isSigner: false,
//               isWritable: true,
//             })),
//           ])
//           .signers([signer])
//           .rpc();
//       } catch (err) {
//         console.log(err.logs);
//         throw err;
//       }

//       const updatedLendertokenAccount = await splToken.getAccount(
//         connection,
//         lenderTokenAccount.address
//       );
//       const updatedBorrowerTokenAccount = await splToken.getAccount(
//         connection,
//         borrower.depositTokenAccount
//       );
//       const tokenManager = await borrower.program.account.tokenManager.fetch(
//         borrower.tokenManager
//       );
//       const defaultedLoan = await borrower.program.account.loan.fetch(
//         borrower.loan
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         rental: false,
//         callOption: false,
//         loan: false,
//       });
//       assert.equal(updatedLendertokenAccount.amount, BigInt(1));
//       assert.equal(updatedBorrowerTokenAccount.amount, BigInt(0));
//       assert.deepEqual(defaultedLoan.state, { defaulted: {} });
//     });
//   });

//   describe("Exercise option with active rental", () => {
//     let seller: helpers.CallOptionSeller;
//     let buyer: helpers.CallOptionBuyer;
//     let rentalTokenAccount: anchor.web3.PublicKey;
//     let thirdPartyKeypair = anchor.web3.Keypair.generate();
//     let callOptionOptions = {
//       amount: 1_000_000,
//       strikePrice: anchor.web3.LAMPORTS_PER_SOL,
//       expiry: Math.round(Date.now() / 1000) + 30 * 24 * 60 * 2, // 2 days
//     };
//     let rentalOptions = {
//       amount: new anchor.BN(anchor.web3.LAMPORTS_PER_SOL / 100),
//       expiry: new anchor.BN(Date.now() / 1000 + 86_400 * 3),
//       borrower: null,
//     };

//     it("Allows active options to be listed for rental", async () => {
//       const signer = await helpers.getSigner();
//       seller = await helpers.askCallOption(connection, callOptionOptions);
//       buyer = await helpers.buyCallOption(connection, seller);

//       const rentalAddress = await helpers.findRentalAddress(
//         seller.mint,
//         seller.keypair.publicKey
//       );
//       await seller.program.methods
//         .initRental(rentalOptions)
//         .accounts({
//           signer: signer.publicKey,
//           rental: rentalAddress,
//           collection: seller.collection,
//           tokenManager: seller.tokenManager,
//           lender: seller.keypair.publicKey,
//           depositTokenAccount: seller.depositTokenAccount,
//           mint: seller.mint,
//           metadata: seller.metatdata,
//           edition: seller.edition,
//           metadataProgram: METADATA_PROGRAM_ID,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           tokenProgram: splToken.TOKEN_PROGRAM_ID,
//           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//         })
//         .signers([signer])
//         .rpc();

//       const callOption = await seller.program.account.callOption.fetch(
//         seller.callOption
//       );
//       const rental = await seller.program.account.rental.fetch(rentalAddress);
//       const tokenManager = await seller.program.account.tokenManager.fetch(
//         seller.tokenManager
//       );
//       const sellerTokenAccount = await splToken.getAccount(
//         connection,
//         seller.depositTokenAccount
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         rental: true,
//         callOption: true,
//         loan: false,
//       });
//       assert.equal(
//         sellerTokenAccount.delegate.toBase58(),
//         seller.tokenManager.toBase58()
//       );
//       assert.equal(
//         callOption.seller.toBase58(),
//         seller.keypair.publicKey.toBase58()
//       );
//       assert.equal(
//         callOption.strikePrice.toNumber(),
//         callOptionOptions.strikePrice
//       );
//       assert.equal(callOption.expiry.toNumber(), callOptionOptions.expiry);
//       assert.equal(callOption.mint.toBase58(), seller.mint.toBase58());
//       assert.deepEqual(callOption.state, { active: {} });
//       assert.equal(sellerTokenAccount.amount, BigInt(1));
//       assert.equal(rental.borrower, null);
//       assert.equal(rental.expiry.toNumber(), rentalOptions.expiry);
//       assert.deepEqual(rental.state, { listed: {} });
//     });

//     it("Allows listed NFTs to be rented", async () => {
//       const signer = await helpers.getSigner();
//       await helpers.requestAirdrop(connection, thirdPartyKeypair.publicKey);
//       const provider = helpers.getProvider(connection, thirdPartyKeypair);
//       const program = helpers.getProgram(provider);
//       const rentalAddress = await helpers.findRentalAddress(
//         seller.mint,
//         seller.keypair.publicKey
//       );
//       const rentalEscrowAddress = await helpers.findRentalEscrowAddress(
//         seller.mint,
//         seller.keypair.publicKey
//       );
//       const tokenManagerAddress = await helpers.findTokenManagerAddress(
//         seller.mint,
//         seller.keypair.publicKey
//       );
//       rentalTokenAccount = await splToken.createAccount(
//         connection,
//         thirdPartyKeypair,
//         seller.mint,
//         thirdPartyKeypair.publicKey
//       );
//       const [metadataAddress] = await helpers.findMetadataAddress(seller.mint);

//       try {
//         await program.methods
//           .takeRental(2)
//           .accounts({
//             signer: signer.publicKey,
//             rentalTokenAccount,
//             borrower: thirdPartyKeypair.publicKey,
//             lender: seller.keypair.publicKey,
//             rental: rentalAddress,
//             rentalEscrow: rentalEscrowAddress,
//             tokenManager: tokenManagerAddress,
//             depositTokenAccount: seller.depositTokenAccount,
//             mint: seller.mint,
//             edition: seller.edition,
//             metadata: metadataAddress,
//             metadataProgram: METADATA_PROGRAM_ID,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: splToken.TOKEN_PROGRAM_ID,
//             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//           })
//           .signers([signer])
//           .rpc();
//       } catch (error) {
//         console.log(error.logs);
//         throw error;
//       }

//       const rental = await seller.program.account.rental.fetch(rentalAddress);
//       const tokenManager = await seller.program.account.tokenManager.fetch(
//         tokenManagerAddress
//       );
//       const tokenAccount = await splToken.getAccount(
//         connection,
//         rentalTokenAccount
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         loan: false,
//         rental: true,
//         callOption: true,
//       });
//       assert.equal(
//         rental.borrower.toBase58(),
//         thirdPartyKeypair.publicKey.toBase58()
//       );
//       assert.deepEqual(rental.state, { rented: {} });
//       assert.equal(tokenAccount.amount, BigInt(1));
//       assert.ok(tokenAccount.isFrozen);
//       assert.ok(tokenAccount.delegate.equals(tokenManagerAddress));
//       assert.equal(tokenAccount.delegatedAmount, BigInt(1));
//     });

//     it("Allows rented NFTs with active call options to be exercised", async () => {
//       const signer = await helpers.getSigner();
//       const rentalAddress = await helpers.findRentalAddress(
//         seller.mint,
//         seller.keypair.publicKey
//       );
//       const rentalEscrowAddress = await helpers.findRentalEscrowAddress(
//         seller.mint,
//         seller.keypair.publicKey
//       );
//       const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
//         connection,
//         buyer.keypair,
//         seller.mint,
//         buyer.keypair.publicKey
//       );
//       const [metadataAddress] = await helpers.findMetadataAddress(seller.mint);
//       const metadata = await Metadata.fromAccountAddress(
//         connection,
//         metadataAddress
//       );
//       const beforeBuyerBalance = await connection.getBalance(
//         buyer.keypair.publicKey
//       );
//       const beforeSellerBalance = await connection.getBalance(
//         seller.keypair.publicKey
//       );

//       let txFee;

//       try {
//         const remainingAccounts = metadata.data.creators
//           .map((creator) => ({
//             pubkey: creator.address,
//             isSigner: false,
//             isWritable: true,
//           }))
//           .concat([
//             {
//               pubkey: thirdPartyKeypair.publicKey,
//               isWritable: true,
//               isSigner: false,
//             },
//           ]);

//         const signature = await buyer.program.methods
//           .exerciseCallOptionWithRental()
//           .accounts({
//             signer: signer.publicKey,
//             seller: seller.keypair.publicKey,
//             buyer: buyer.keypair.publicKey,
//             callOption: seller.callOption,
//             rental: rentalAddress,
//             rentalEscrow: rentalEscrowAddress,
//             tokenManager: seller.tokenManager,
//             buyerTokenAccount: tokenAccount.address,
//             tokenAccount: rentalTokenAccount,
//             mint: seller.mint,
//             edition: seller.edition,
//             metadata: metadataAddress,
//             metadataProgram: METADATA_PROGRAM_ID,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: splToken.TOKEN_PROGRAM_ID,
//             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//             rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//           })
//           .remainingAccounts(remainingAccounts)
//           .signers([signer])
//           .rpc();

//         const latestBlockhash = await connection.getLatestBlockhash();
//         await connection.confirmTransaction(
//           {
//             signature,
//             ...latestBlockhash,
//           },
//           "confirmed"
//         );
//         const tx = await connection.getTransaction(signature, {
//           commitment: "confirmed",
//           maxSupportedTransactionVersion: 0,
//         });
//         txFee = tx.meta.fee;
//       } catch (err) {
//         console.log(err.logs);
//         assert.fail(err);
//       }

//       const afterBuyerBalance = await connection.getBalance(
//         buyer.keypair.publicKey
//       );
//       const afterSellerBalance = await connection.getBalance(
//         seller.keypair.publicKey
//       );
//       const callOption = await seller.program.account.callOption.fetch(
//         seller.callOption
//       );
//       const rentalAccount = await connection.getAccountInfo(rentalAddress);
//       const tokenManager = await seller.program.account.tokenManager.fetch(
//         seller.tokenManager
//       );
//       const buyerTokenAccount = await splToken.getAccount(
//         connection,
//         tokenAccount.address
//       );
//       const creatorFees =
//         (metadata.data.sellerFeeBasisPoints / 10_000) *
//         callOption.strikePrice.toNumber();

//       const estimatedBuyerBalance =
//         beforeBuyerBalance -
//         callOptionOptions.strikePrice -
//         creatorFees -
//         txFee;
//       const estimatedSellerBalance =
//         beforeSellerBalance + callOptionOptions.strikePrice;

//       assert.equal(estimatedBuyerBalance, afterBuyerBalance, "buyer balance");
//       assert.ok(afterSellerBalance >= estimatedSellerBalance, "seller balance");
//       assert.deepEqual(callOption.state, { exercised: {} });
//       assert.equal(buyerTokenAccount.amount, BigInt(1));
//       assert.equal(rentalAccount, null);
//       assert.deepEqual(tokenManager.accounts, {
//         rental: false,
//         callOption: false,
//         loan: false,
//       });
//     });
//   });

//   describe("List call option after active rental", () => {
//     let lender: helpers.RentalLender;
//     let borrower: helpers.RentalBorrower;

//     let rentalOptions = {
//       amount: anchor.web3.LAMPORTS_PER_SOL / 100,
//       expiry: Math.round(Date.now() / 1000 + 86_400 * 3),
//       borrower: null,
//     };

//     it("Lists a rental", async () => {
//       lender = await helpers.initRental(connection, rentalOptions);

//       const rental = await lender.program.account.rental.fetch(lender.rental);
//       const tokenManager = await lender.program.account.tokenManager.fetch(
//         lender.tokenManager
//       );
//       const tokenAccount = await splToken.getAccount(
//         connection,
//         lender.depositTokenAccount
//       );
//       assert.deepEqual(tokenManager.accounts, {
//         rental: true,
//         callOption: false,
//         loan: false,
//       });
//       assert.equal(
//         tokenAccount.delegate.toBase58(),
//         lender.tokenManager.toBase58()
//       );
//       assert.equal(tokenAccount.amount, BigInt(1));
//       assert.equal(rental.borrower, null);
//       assert.equal(rental.expiry.toNumber(), rentalOptions.expiry);
//       assert.deepEqual(rental.state, { listed: {} });
//     });

//     it("Allows listed NFTs to be rented", async () => {
//       borrower = await helpers.takeRental(connection, lender, 2);

//       const rental = await lender.program.account.rental.fetch(lender.rental);
//       const tokenManager = await lender.program.account.tokenManager.fetch(
//         lender.tokenManager
//       );
//       const tokenAccount = await splToken.getAccount(
//         connection,
//         borrower.rentalTokenAccount
//       );

//       assert.deepEqual(tokenManager.accounts, {
//         loan: false,
//         rental: true,
//         callOption: false,
//       });
//       assert.equal(
//         rental.borrower.toBase58(),
//         borrower.keypair.publicKey.toBase58()
//       );
//       assert.deepEqual(rental.state, { rented: {} });
//       assert.equal(tokenAccount.amount, BigInt(1));
//       assert.ok(tokenAccount.isFrozen);
//       assert.ok(tokenAccount.delegate.equals(lender.tokenManager));
//       assert.equal(tokenAccount.delegatedAmount, BigInt(1));
//     });

//     it("Restricts call option creation to original lender", async () => {
//       const signer = await helpers.getSigner();
//       const amount = new anchor.BN(1_000_000);
//       const strikePrice = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL);
//       const expiry = new anchor.BN(
//         Math.round(Date.now() / 1000 + 30 * 24 * 60 * 2)
//       );
//       const callOptionAddress = await helpers.findCallOptionAddress(
//         lender.mint,
//         borrower.keypair.publicKey
//       );
//       const tokenManager = await helpers.findTokenManagerAddress(
//         lender.mint,
//         borrower.keypair.publicKey
//       );

//       try {
//         await borrower.program.methods
//           .askCallOption(amount, strikePrice, expiry)
//           .accounts({
//             signer: signer.publicKey,
//             tokenManager,
//             callOption: callOptionAddress,
//             collection: lender.collection,
//             depositTokenAccount: borrower.rentalTokenAccount,
//             mint: lender.mint,
//             metadata: lender.metadata,
//             edition: lender.edition,
//             seller: borrower.keypair.publicKey,
//             metadataProgram: METADATA_PROGRAM_ID,
//             tokenProgram: splToken.TOKEN_PROGRAM_ID,
//             rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//           })
//           .signers([signer])
//           .rpc();
//         assert.fail("Expected to throw");
//       } catch (err) {
//         assert(err instanceof anchor.AnchorError);
//         assert.equal(err.error.errorCode.number, 6014);
//         assert.equal(err.error.errorCode.code, "InvalidDelegate");
//       }
//     });

//     it("Allows active rentals to be listed as call options", async () => {
//       const signer = await helpers.getSigner();
//       const callOptionAddress = await helpers.findCallOptionAddress(
//         lender.mint,
//         lender.keypair.publicKey
//       );
//       const tokenManager = await helpers.findTokenManagerAddress(
//         lender.mint,
//         lender.keypair.publicKey
//       );

//       const amount = new anchor.BN(1_000_000);
//       const strikePrice = new anchor.BN(anchor.web3.LAMPORTS_PER_SOL);
//       const expiry = new anchor.BN(
//         Math.round(Date.now() / 1000 + 30 * 24 * 60 * 2)
//       );

//       await lender.program.methods
//         .askCallOption(amount, strikePrice, expiry)
//         .accounts({
//           signer: signer.publicKey,
//           tokenManager,
//           callOption: callOptionAddress,
//           collection: lender.collection,
//           depositTokenAccount: borrower.rentalTokenAccount,
//           mint: lender.mint,
//           metadata: lender.metadata,
//           edition: lender.edition,
//           seller: lender.keypair.publicKey,
//           metadataProgram: METADATA_PROGRAM_ID,
//           tokenProgram: splToken.TOKEN_PROGRAM_ID,
//           rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
//         })
//         .signers([signer])
//         .rpc();

//       const callOption = await lender.program.account.callOption.fetch(
//         callOptionAddress
//       );
//       const tokenManagerData = await lender.program.account.tokenManager.fetch(
//         lender.tokenManager
//       );
//       const tokenAccount = await splToken.getAccount(
//         connection,
//         borrower.rentalTokenAccount
//       );

//       assert.deepEqual(tokenManagerData.accounts, {
//         rental: true,
//         callOption: true,
//         loan: false,
//       });
//       assert.equal(
//         tokenAccount.delegate.toBase58(),
//         lender.tokenManager.toBase58()
//       );
//       assert.equal(
//         callOption.seller.toBase58(),
//         lender.keypair.publicKey.toBase58()
//       );
//       assert.equal(callOption.strikePrice.toNumber(), strikePrice.toNumber());
//       assert.equal(callOption.expiry.toNumber(), expiry.toNumber());
//       assert.equal(callOption.mint.toBase58(), lender.mint.toBase58());
//       assert.deepEqual(callOption.state, { listed: {} });
//       assert.equal(tokenAccount.amount, BigInt(1));
//     });
//   });
// });
