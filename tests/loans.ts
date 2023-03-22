require("dotenv").config();

import assert from "assert";
import {
  Metadata,
  TokenRecord,
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

describe.only("Loans", () => {
  describe("Offers", () => {
    let lender: helpers.LoanOfferLender;
    let borrower: helpers.LoanOfferBorrower;
    let options;

    it("Creates an offer", async () => {
      options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 86_400,
      };

      lender = await helpers.offerLoan(connection, options);

      const offer = await lender.program.account.loanOffer.fetch(
        lender.loanOffer
      );
      assert.equal(offer.amount.toNumber(), options.amount);
    });

    it("Takes an offer", async () => {
      borrower = await helpers.takeLoan(connection, lender);

      try {
        await lender.program.account.loanOffer.fetch(borrower.loanOffer);
      } catch (err) {
        assert.ok(err.message.includes("Account does not exist"));
      }
      const loan = await lender.program.account.loan.fetch(borrower.loan);
      const tokenRecord = await TokenRecord.fromAccountAddress(
        connection,
        borrower.tokenRecord
      );
      const tokenAccount = await splToken.getAccount(
        connection,
        borrower.depositTokenAccount
      );
      const borrowerLamportsAfter = (
        await connection.getAccountInfo(borrower.keypair.publicKey)
      ).lamports;
      assert.ok(
        tokenRecord.lockedTransfer.equals(borrower.tokenManager),
        "lockedTransfer"
      );
      assert.ok(loan.borrower.equals(borrower.keypair.publicKey), "borrower");
      assert.ok(loan.lender.equals(lender.keypair.publicKey), "lender");
      assert.equal(loan.amount.toNumber(), options.amount, "amount");
      assert.equal(loan.duration.toNumber(), options.duration, "duration");
      assert.ok(loan.startDate.toNumber() > 0, "startDate");
      assert.ok(tokenAccount.isFrozen, "isFrozen");
      assert.ok(
        tokenAccount.delegate.equals(borrower.tokenManager),
        "delegate"
      );
      assert.ok(
        borrowerLamportsAfter > anchor.web3.LAMPORTS_PER_SOL * 3 - 9_000_000,
        "lamports"
      );
    });

    it("Closes an offer", async () => {
      const signer = await helpers.getSigner();
      const offer = await helpers.offerLoan(connection, {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 86_400,
      });

      await offer.program.methods
        .closeLoanOffer(offer.id)
        .accounts({
          signer: signer.publicKey,
          lender: offer.keypair.publicKey,
          loanOffer: offer.loanOffer,
          escrowPaymentAccount: offer.escrowPaymentAccount,
          collection: offer.collection,
        })
        .signers([signer])
        .rpc();
    });
  });

  describe("Loan repossessions", () => {
    let borrower: helpers.LoanBorrower;
    let lender: helpers.LoanLender;
    let options;

    it("Creates a dexloan loan", async () => {
      options = {
        amount: anchor.web3.LAMPORTS_PER_SOL / 100,
        basisPoints: 500,
        duration: 1, // 1 second
      };

      borrower = await helpers.askLoan(connection, options);

      const borrowerTokenAccount = await splToken.getAccount(
        connection,
        borrower.depositTokenAccount
      );
      const loan = await borrower.program.account.loan.fetch(borrower.loan);
      const tokenManager = await borrower.program.account.tokenManager.fetch(
        borrower.tokenManager
      );

      assert.deepEqual(tokenManager.accounts, {
        rental: false,
        callOption: false,
        loan: true,
      });
      assert.equal(
        borrowerTokenAccount.delegate.toBase58(),
        borrower.tokenManager.toBase58()
      );
      assert.equal(
        loan.borrower.toBase58(),
        borrower.keypair.publicKey.toBase58()
      );
      assert.equal(loan.basisPoints, options.basisPoints);
      assert.equal(loan.duration.toNumber(), options.duration);
      assert.equal(loan.mint.toBase58(), borrower.mint.toBase58());
      assert.equal(borrowerTokenAccount.amount, BigInt(1));
      assert.deepEqual(loan.state, { listed: {} });
    });

    it("Freezes tokens after initialization", async () => {
      const receiver = anchor.web3.Keypair.generate();
      await helpers.requestAirdrop(connection, receiver.publicKey);

      const receiverTokenAccount = await splToken.createAccount(
        connection,
        receiver,
        borrower.mint,
        receiver.publicKey
      );

      try {
        await splToken.transfer(
          connection,
          borrower.keypair,
          borrower.depositTokenAccount,
          receiverTokenAccount,
          borrower.keypair.publicKey,
          1
        );
        assert.ok(false);
      } catch (err) {
        assert.ok(err.logs.includes("Program log: Error: Account is frozen"));
      }
    });

    it("Allows loans to be given", async () => {
      const borrowerPreLoanBalance = await connection.getBalance(
        borrower.keypair.publicKey
      );

      lender = await helpers.giveLoan(connection, borrower);
      const loan = await borrower.program.account.loan.fetch(borrower.loan);
      const tokenManager = await borrower.program.account.tokenManager.fetch(
        borrower.tokenManager
      );
      const borrowerPostLoanBalance = await connection.getBalance(
        borrower.keypair.publicKey
      );
      const borrowerTokenAccount = await splToken.getAccount(
        connection,
        borrower.depositTokenAccount
      );

      assert.deepEqual(tokenManager.accounts, {
        rental: false,
        callOption: false,
        loan: true,
      });
      assert.equal(borrowerTokenAccount.amount, BigInt(1));
      assert.equal(
        borrowerPreLoanBalance + options.amount,
        borrowerPostLoanBalance
      );
      assert.equal(loan.lender.toBase58(), lender.keypair.publicKey.toBase58());
      assert.deepEqual(loan.state, { active: {} });
      assert(
        loan.startDate.toNumber() > 0 && loan.startDate.toNumber() < Date.now()
      );
    });

    //   it("Will only allow lender to repossess an overdue loan", async () => {
    //     // Creates another signer
    //     const keypair = anchor.web3.Keypair.generate();
    //     const signer = await helpers.getSigner();
    //     const provider = helpers.getProvider(connection, keypair);
    //     const program = helpers.getProgram(provider);
    //     await helpers.requestAirdrop(connection, keypair.publicKey);

    //     const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
    //       connection,
    //       keypair,
    //       borrower.mint,
    //       keypair.publicKey
    //     );

    //     try {
    //       await program.methods
    //         .repossess()
    //         .accounts({
    //           signer: signer.publicKey,
    //           borrower: borrower.keypair.publicKey,
    //           depositTokenAccount: borrower.depositTokenAccount,
    //           lender: lender.keypair.publicKey,
    //           lenderTokenAccount: tokenAccount.address,
    //           loan: borrower.loan,
    //           tokenManager: borrower.tokenManager,
    //           mint: borrower.mint,
    //           edition: borrower.edition,
    //           metadataProgram: METADATA_PROGRAM_ID,
    //           systemProgram: anchor.web3.SystemProgram.programId,
    //           tokenProgram: splToken.TOKEN_PROGRAM_ID,
    //           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
    //           rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    //         })
    //         .signers([signer])
    //         .rpc();

    //       assert.ok(false);
    //     } catch (error) {
    //       assert.ok(
    //         error.toString().includes("Error: Signature verification failed")
    //       );
    //     }
    //   });

    //   it("Allows an overdue loan to be repossessed by the lender", async () => {
    //     const signer = await helpers.getSigner();
    //     const loan = await borrower.program.account.loan.fetch(borrower.loan);
    //     const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
    //       connection,
    //       lender.keypair,
    //       loan.mint,
    //       lender.keypair.publicKey
    //     );

    //     try {
    //       await lender.program.methods
    //         .repossess()
    //         .accounts({
    //           signer: signer.publicKey,
    //           borrower: borrower.keypair.publicKey,
    //           depositTokenAccount: borrower.depositTokenAccount,
    //           lender: lender.keypair.publicKey,
    //           lenderTokenAccount: tokenAccount.address,
    //           loan: borrower.loan,
    //           tokenManager: borrower.tokenManager,
    //           mint: loan.mint,
    //           edition: borrower.edition,
    //           metadataProgram: METADATA_PROGRAM_ID,
    //           systemProgram: anchor.web3.SystemProgram.programId,
    //           tokenProgram: splToken.TOKEN_PROGRAM_ID,
    //           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
    //           rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    //         })
    //         .signers([signer])
    //         .rpc();
    //     } catch (err) {
    //       console.log(err.logs);
    //       throw err;
    //     }

    //     const lenderTokenAccount = await splToken.getAccount(
    //       connection,
    //       tokenAccount.address
    //     );
    //     const tokenManager = await borrower.program.account.tokenManager.fetch(
    //       borrower.tokenManager
    //     );
    //     const defaultedListing = await borrower.program.account.loan.fetch(
    //       borrower.loan
    //     );

    //     assert.deepEqual(tokenManager.accounts, {
    //       rental: false,
    //       callOption: false,
    //       loan: false,
    //     });
    //     assert.equal(lenderTokenAccount.amount, BigInt(1));
    //     assert.deepEqual(defaultedListing.state, { defaulted: {} });
    //   });

    //   it("Will allow accounts to be closed once overdue loans are repossessed", async () => {
    //     const signer = await helpers.getSigner();

    //     try {
    //       await borrower.program.methods
    //         .closeLoan()
    //         .accounts({
    //           signer: signer.publicKey,
    //           borrower: borrower.keypair.publicKey,
    //           depositTokenAccount: borrower.depositTokenAccount,
    //           loan: borrower.loan,
    //           tokenManager: borrower.tokenManager,
    //           mint: borrower.mint,
    //           edition: borrower.edition,
    //           metadataProgram: METADATA_PROGRAM_ID,
    //           systemProgram: anchor.web3.SystemProgram.programId,
    //           tokenProgram: splToken.TOKEN_PROGRAM_ID,
    //         })
    //         .signers([signer])
    //         .rpc();
    //     } catch (err) {
    //       console.log(err.logs);
    //       assert.fail(err);
    //     }

    //     try {
    //       await borrower.program.account.loan.fetch(borrower.loan);
    //     } catch (err) {
    //       assert.equal(
    //         err.message,
    //         `Account does not exist or has no data ${borrower.loan.toBase58()}`
    //       );
    //     }
    //   });
  });

  // describe("Loan repayments", () => {
  //   let borrower: Awaited<ReturnType<typeof helpers.askLoan>>;
  //   let lender: Awaited<ReturnType<typeof helpers.giveLoan>>;
  //   let options;

  //   it("Creates a dexloan loan", async () => {
  //     options = {
  //       amount: anchor.web3.LAMPORTS_PER_SOL / 10,
  //       basisPoints: 5000,
  //       duration: 30 * 24 * 60 * 60, // 30 days
  //     };

  //     borrower = await helpers.askLoan(connection, options);
  //     const borrowerTokenAccount = await splToken.getAccount(
  //       connection,
  //       borrower.depositTokenAccount
  //     );
  //     const loan = await borrower.program.account.loan.fetch(borrower.loan);
  //     const tokenManager = await borrower.program.account.tokenManager.fetch(
  //       borrower.tokenManager
  //     );

  //     assert.deepEqual(tokenManager.accounts, {
  //       rental: false,
  //       callOption: false,
  //       loan: true,
  //     });
  //     assert.equal(
  //       borrowerTokenAccount.delegate.toBase58(),
  //       borrower.tokenManager.toBase58()
  //     );
  //     assert.equal(
  //       loan.borrower.toBase58(),
  //       borrower.keypair.publicKey.toBase58()
  //     );
  //     assert.equal(loan.basisPoints, options.basisPoints);
  //     assert.equal(loan.duration.toNumber(), options.duration);
  //     assert.equal(loan.mint.toBase58(), borrower.mint.toBase58());
  //     assert.equal(borrowerTokenAccount.amount, BigInt(1));
  //     assert.deepEqual(loan.state, { listed: {} });
  //   });

  //   it("Prevents reinitialization", async () => {
  //     const amount = anchor.web3.LAMPORTS_PER_SOL;
  //     const basisPoints = 500;

  //     try {
  //       const signer = await helpers.getSigner();

  //       await borrower.program.methods
  //         .askLoan(new anchor.BN(amount), basisPoints, new anchor.BN(1))
  //         .accounts({
  //           signer: signer.publicKey,
  //           loan: borrower.loan,
  //           collection: borrower.collection,
  //           tokenManager: borrower.tokenManager,
  //           depositTokenAccount: borrower.depositTokenAccount,
  //           mint: borrower.mint,
  //           metadata: borrower.metadata,
  //           borrower: borrower.keypair.publicKey,
  //           edition: borrower.edition,
  //           metadataProgram: METADATA_PROGRAM_ID,
  //           tokenProgram: splToken.TOKEN_PROGRAM_ID,
  //           rent: anchor.web3.SYSVAR_RENT_PUBKEY,
  //           systemProgram: anchor.web3.SystemProgram.programId,
  //         })
  //         .signers([signer])
  //         .rpc();
  //       assert.fail();
  //     } catch (error) {
  //       assert.ok(error.toString().includes("custom program error: 0x0"));
  //     }
  //   });

  //   it("Allows unactive loans to be closed", async () => {
  //     try {
  //       const signer = await helpers.getSigner();

  //       await borrower.program.methods
  //         .closeLoan()
  //         .accounts({
  //           signer: signer.publicKey,
  //           loan: borrower.loan,
  //           tokenManager: borrower.tokenManager,
  //           borrower: borrower.keypair.publicKey,
  //           depositTokenAccount: borrower.depositTokenAccount,
  //           mint: borrower.mint,
  //           edition: borrower.edition,
  //           metadataProgram: METADATA_PROGRAM_ID,
  //           systemProgram: anchor.web3.SystemProgram.programId,
  //           tokenProgram: splToken.TOKEN_PROGRAM_ID,
  //         })
  //         .signers([signer])
  //         .rpc();
  //     } catch (error) {
  //       console.log(error.logs);
  //       assert.fail(error);
  //     }

  //     const borrowerTokenAccount = await splToken.getAccount(
  //       connection,
  //       borrower.depositTokenAccount
  //     );
  //     assert.equal(borrowerTokenAccount.delegate, null);
  //     assert.equal(borrowerTokenAccount.amount, BigInt(1));
  //   });

  //   it("Allows loans to be reinitialized after being closed", async () => {
  //     const amount = new anchor.BN(options.amount);
  //     const basisPoints = options.basisPoints;
  //     const duration = new anchor.BN(options.duration);

  //     const signer = await helpers.getSigner();

  //     await borrower.program.methods
  //       .askLoan(amount, basisPoints, duration)
  //       .accounts({
  //         signer: signer.publicKey,
  //         loan: borrower.loan,
  //         collection: borrower.collection,
  //         tokenManager: borrower.tokenManager,
  //         depositTokenAccount: borrower.depositTokenAccount,
  //         metadata: borrower.metadata,
  //         mint: borrower.mint,
  //         borrower: borrower.keypair.publicKey,
  //         edition: borrower.edition,
  //         metadataProgram: METADATA_PROGRAM_ID,
  //         tokenProgram: splToken.TOKEN_PROGRAM_ID,
  //         rent: anchor.web3.SYSVAR_RENT_PUBKEY,
  //         systemProgram: anchor.web3.SystemProgram.programId,
  //       })
  //       .signers([signer])
  //       .rpc();

  //     const loan = await borrower.program.account.loan.fetch(borrower.loan);
  //     const borrowerTokenAccount = await splToken.getAccount(
  //       connection,
  //       borrower.depositTokenAccount
  //     );

  //     assert.equal(borrowerTokenAccount.amount, BigInt(1));
  //     assert.equal(
  //       borrowerTokenAccount.delegate.toBase58(),
  //       borrower.tokenManager.toBase58()
  //     );
  //     assert.deepEqual(loan.state, { listed: {} });
  //     assert.equal(
  //       loan.borrower.toBase58(),
  //       borrower.keypair.publicKey.toBase58()
  //     );
  //   });

  //   it("Allows loans to be given", async () => {
  //     const borrowerPreLoanBalance = await connection.getBalance(
  //       borrower.keypair.publicKey
  //     );

  //     lender = await helpers.giveLoan(connection, borrower);
  //     const loan = await borrower.program.account.loan.fetch(borrower.loan);
  //     const borrowerPostLoanBalance = await connection.getBalance(
  //       borrower.keypair.publicKey
  //     );
  //     const borrowerTokenAccount = await splToken.getAccount(
  //       connection,
  //       borrower.depositTokenAccount
  //     );
  //     const tokenManager = await borrower.program.account.tokenManager.fetch(
  //       borrower.tokenManager
  //     );

  //     assert.deepEqual(tokenManager.accounts, {
  //       rental: false,
  //       callOption: false,
  //       loan: true,
  //     });
  //     assert.equal(borrowerTokenAccount.amount, BigInt(1));
  //     assert.equal(
  //       borrowerPreLoanBalance + options.amount,
  //       borrowerPostLoanBalance
  //     );
  //     assert.equal(loan.lender.toBase58(), lender.keypair.publicKey.toBase58());
  //     assert.deepEqual(loan.state, { active: {} });
  //     assert(
  //       loan.startDate.toNumber() > 0 && loan.startDate.toNumber() < Date.now()
  //     );
  //   });

  //   it("Will not allow a loan to be repossessed if not overdue", async () => {
  //     const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
  //       connection,
  //       lender.keypair,
  //       borrower.mint,
  //       lender.keypair.publicKey
  //     );

  //     try {
  //       const signer = await helpers.getSigner();

  //       await lender.program.methods
  //         .repossess()
  //         .accounts({
  //           signer: signer.publicKey,
  //           borrower: borrower.keypair.publicKey,
  //           depositTokenAccount: borrower.depositTokenAccount,
  //           lender: lender.keypair.publicKey,
  //           lenderTokenAccount: tokenAccount.address,
  //           loan: borrower.loan,
  //           tokenManager: borrower.tokenManager,
  //           mint: borrower.mint,
  //           edition: borrower.edition,
  //           metadataProgram: METADATA_PROGRAM_ID,
  //           systemProgram: anchor.web3.SystemProgram.programId,
  //           tokenProgram: splToken.TOKEN_PROGRAM_ID,
  //           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
  //           rent: anchor.web3.SYSVAR_RENT_PUBKEY,
  //         })
  //         .signers([signer])
  //         .rpc();

  //       assert.ok(false);
  //     } catch (error) {
  //       assert.ok(error.toString(), "This loan is not overdue");
  //     }
  //   });

  //   it("Allows loans to be repaid", async () => {
  //     const signer = await helpers.getSigner();
  //     const borrower = await helpers.askLoan(connection, options);
  //     const lender = await helpers.giveLoan(connection, borrower);
  //     const lenderPreRepaymentBalance = await connection.getBalance(
  //       lender.keypair.publicKey
  //     );
  //     const metadata = await Metadata.fromAccountAddress(
  //       connection,
  //       borrower.metadata
  //     );
  //     const loan = await borrower.program.account.loan.fetch(borrower.loan);
  //     await helpers.wait(10);

  //     const signature = await borrower.program.methods
  //       .repayLoan()
  //       .accounts({
  //         signer: signer.publicKey,
  //         loan: borrower.loan,
  //         tokenManager: borrower.tokenManager,
  //         borrower: borrower.keypair.publicKey,
  //         depositTokenAccount: borrower.depositTokenAccount,
  //         lender: lender.keypair.publicKey,
  //         mint: borrower.mint,
  //         metadata: borrower.metadata,
  //         edition: borrower.edition,
  //         metadataProgram: METADATA_PROGRAM_ID,
  //         systemProgram: anchor.web3.SystemProgram.programId,
  //         tokenProgram: splToken.TOKEN_PROGRAM_ID,
  //         clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
  //       })
  //       .remainingAccounts(
  //         metadata.data.creators.map((creator) => ({
  //           pubkey: creator.address,
  //           isSigner: false,
  //           isWritable: true,
  //         }))
  //       )
  //       .signers([signer])
  //       .rpc({ commitment: "confirmed" });
  //     const tx = await connection.getTransaction(signature, {
  //       commitment: "confirmed",
  //     });
  //     const lenderPostRepaymentBalance = await connection.getBalance(
  //       lender.keypair.publicKey
  //     );
  //     const estimatedInterest = Math.floor(
  //       (((loan.basisPoints * loan.amount.toNumber()) / 10_000) *
  //         (tx.blockTime - loan.startDate.toNumber())) /
  //         31_536_000
  //     );
  //     const estimatedLenderBalance =
  //       lenderPreRepaymentBalance + loan.amount.toNumber() + estimatedInterest;
  //     const borrowerTokenAccount = await splToken.getAccount(
  //       connection,
  //       borrower.depositTokenAccount
  //     );

  //     try {
  //       await borrower.program.account.tokenManager.fetch(
  //         borrower.tokenManager
  //       );
  //     } catch (error) {
  //       assert.ok(
  //         error.toString(),
  //         `Account does not exist ${borrower.tokenManager.toBase58()}`
  //       );
  //     }

  //     assert.equal(borrowerTokenAccount.amount, BigInt(1));
  //     assert.equal(borrowerTokenAccount.delegate, null);
  //     assert.equal(
  //       lenderPostRepaymentBalance,
  //       estimatedLenderBalance,
  //       "Lender Balance"
  //     );
  //   });
  // });
});
