import assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as splToken from "@solana/spl-token";
import * as helpers from "./helpers";

describe("dexloan_listings", () => {
  // Configure the client to use the local cluster.
  const connection = new anchor.web3.Connection(
    "http://127.0.0.1:8899",
    anchor.AnchorProvider.defaultOptions().preflightCommitment
  );

  describe("Loans", () => {
    it("Creates a dexloan listing", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 30 * 24 * 60 * 60, // 30 days
      };
      const borrower = await helpers.initLoan(connection, options);

      const listing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );
      const borrowerTokenAccount = await splToken.getAccount(
        connection,
        borrower.associatedAddress
      );
      const escrowTokenAccount = await splToken.getAccount(
        connection,
        listing.escrow
      );

      assert.equal(borrowerTokenAccount.delegate, listing.escrow.toBase58());
      assert.equal(listing.borrower, borrower.keypair.publicKey.toString());
      assert.equal(listing.basisPoints, options.basisPoints);
      assert.equal(listing.duration.toNumber(), options.duration);
      assert.equal(listing.mint.toBase58(), borrower.mint.toBase58());
      assert.equal(borrowerTokenAccount.amount, BigInt(1));
      assert.equal(escrowTokenAccount.amount, BigInt(0));
      assert.equal(
        escrowTokenAccount.mint.toBase58(),
        borrower.mint.toBase58()
      );
      assert.equal(listing.state, 1);
      assert.equal(
        escrowTokenAccount.owner.toBase58(),
        borrower.escrowAccount.toBase58()
      );
    });

    it("Allows loans to be given", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 30 * 24 * 60 * 60, // 30 days
      };
      const borrower = await helpers.initLoan(connection, options);
      const borrowerPreLoanBalance = await connection.getBalance(
        borrower.keypair.publicKey
      );

      const lender = await helpers.createLoan(connection, borrower);

      const listing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );
      const borrowerPostLoanBalance = await connection.getBalance(
        borrower.keypair.publicKey
      );
      const borrowerTokenAccount = await splToken.getAccount(
        connection,
        borrower.associatedAddress
      );
      const escrowTokenAccount = await splToken.getAccount(
        connection,
        listing.escrow
      );

      assert.equal(borrowerTokenAccount.amount, BigInt(0));
      assert.equal(escrowTokenAccount.amount, BigInt(1));
      assert.equal(
        borrowerPreLoanBalance + options.amount,
        borrowerPostLoanBalance
      );
      assert.equal(
        listing.lender.toBase58(),
        lender.keypair.publicKey.toBase58()
      );
      assert.equal(listing.state, 2);
      assert(
        listing.startDate.toNumber() > 0 &&
          listing.startDate.toNumber() < Date.now()
      );
    });

    it("Allows loans to be repaid", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL * 2,
        basisPoints: 700,
        duration: 30 * 24 * 60 * 60, // 30 days
      };
      const borrower = await helpers.initLoan(connection, options);
      const lender = await helpers.createLoan(connection, borrower);
      const lenderPreRepaymentBalance = await connection.getBalance(
        lender.keypair.publicKey
      );

      await borrower.program.methods
        .repayLoan()
        .accounts({
          listingAccount: borrower.listingAccount,
          escrowAccount: borrower.escrowAccount,
          borrower: borrower.keypair.publicKey,
          depositTokenAccount: borrower.associatedAddress,
          lender: lender.keypair.publicKey,
          mint: borrower.mint,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        })
        .rpc();

      const lenderPostRepaymentBalance = await connection.getBalance(
        lender.keypair.publicKey
      );
      const borrowerTokenAccount = await splToken.getAccount(
        connection,
        borrower.associatedAddress
      );
      const escrowTokenAccount = await splToken.getAccount(
        connection,
        borrower.escrowAccount
      );

      assert.equal(borrowerTokenAccount.amount, BigInt(1));
      assert.equal(escrowTokenAccount.amount, BigInt(0));
      assert(lenderPostRepaymentBalance > lenderPreRepaymentBalance);
    });

    it("Allows listings to be cancelled", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 30 * 24 * 60 * 60, // 30 days
      };
      const borrower = await helpers.initLoan(connection, options);

      try {
        await borrower.program.methods
          .cancelListing()
          .accounts({
            listingAccount: borrower.listingAccount,
            escrowAccount: borrower.escrowAccount,
            borrower: borrower.keypair.publicKey,
            depositTokenAccount: borrower.associatedAddress,
            mint: borrower.mint,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: splToken.TOKEN_PROGRAM_ID,
          })
          .rpc();
      } catch (error) {
        console.log(error.logs);
        assert.fail(error);
      }

      const borrowerTokenAccount = await splToken.getAccount(
        connection,
        borrower.associatedAddress
      );
      const escrowTokenAccount = await splToken.getAccount(
        connection,
        borrower.escrowAccount
      );
      assert.equal(borrowerTokenAccount.delegate, null);
      assert.equal(borrowerTokenAccount.amount, BigInt(1));
      assert.equal(escrowTokenAccount.amount, BigInt(0));
    });

    it("Allows listings to be reinitialized after being cancelled", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 30 * 24 * 60 * 60, // 30 days
      };
      const borrower = await helpers.initLoan(connection, options);

      await borrower.program.methods
        .cancelListing()
        .accounts({
          listingAccount: borrower.listingAccount,
          escrowAccount: borrower.escrowAccount,
          borrower: borrower.keypair.publicKey,
          depositTokenAccount: borrower.associatedAddress,
          mint: borrower.mint,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
        })
        .rpc();

      const amount = new anchor.BN(options.amount);
      const basisPoints = new anchor.BN(options.basisPoints);
      const duration = new anchor.BN(options.duration);

      await borrower.program.methods
        .initLoan(amount, basisPoints, duration)
        .accounts({
          escrowAccount: borrower.escrowAccount,
          listingAccount: borrower.listingAccount,
          borrower: borrower.keypair.publicKey,
          depositTokenAccount: borrower.associatedAddress,
          mint: borrower.mint,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      const listing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );
      const borrowerTokenAccount = await splToken.getAccount(
        connection,
        borrower.associatedAddress
      );
      const escrowTokenAccount = await splToken.getAccount(
        connection,
        borrower.escrowAccount
      );

      assert.equal(borrowerTokenAccount.amount, BigInt(1));
      assert.equal(escrowTokenAccount.amount, BigInt(0));
      assert.equal(
        borrowerTokenAccount.delegate.toBase58(),
        listing.escrow.toBase58()
      );
      assert.equal(listing.state, 1);
      assert.equal(
        listing.borrower.toBase58(),
        borrower.keypair.publicKey.toBase58()
      );
    });

    it("Does NOT allow an active listing to be reinitialized", async () => {
      const amount = anchor.web3.LAMPORTS_PER_SOL;
      const basisPoints = 500;
      const duration = 60;

      const borrower = await helpers.initLoan(connection, {
        amount,
        basisPoints,
        duration,
      });
      await helpers.createLoan(connection, borrower);

      const listing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );

      try {
        await borrower.program.methods
          .initLoan(amount, basisPoints, duration)
          .accounts({
            borrower: borrower.keypair.publicKey,
            depositTokenAccount: borrower.associatedAddress,
            escrowAccount: listing.escrow,
            listingAccount: borrower.listingAccount,
            mint: listing.mint,
            tokenProgram: splToken.TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        assert.ok(false);
      } catch (error) {
        assert.ok(error.toString().includes("custom program error: 0x0"));
      }
    });

    it("Allows an overdue loan to be repossessed", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 1, // 1 second
      };
      const borrower = await helpers.initLoan(connection, options);

      const lender = await helpers.createLoan(connection, borrower);

      await wait(1); // ensure 1 second passes

      const listing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );

      const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
        connection,
        lender.keypair,
        listing.mint,
        lender.keypair.publicKey
      );

      await lender.program.methods
        .repossessCollateral()
        .accounts({
          escrowAccount: borrower.escrowAccount,
          lender: lender.keypair.publicKey,
          lenderTokenAccount: tokenAccount.address,
          listingAccount: borrower.listingAccount,
          mint: listing.mint,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      const escrowTokenAccount = await splToken.getAccount(
        connection,
        listing.escrow
      );
      const lenderTokenAccount = await splToken.getAccount(
        connection,
        tokenAccount.address
      );
      const defaultedListing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );
      assert.equal(escrowTokenAccount.amount, BigInt(0));
      assert.equal(lenderTokenAccount.amount, BigInt(1));
      assert.equal(defaultedListing.state, 5);
    });

    it("Will allow accounts to be closed once overdue loans are repossessed", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 1, // 1 second
      };
      const borrower = await helpers.initLoan(connection, options);

      const lender = await helpers.createLoan(connection, borrower);

      await wait(1); // ensure 1 second passes

      const listing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );

      const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
        connection,
        lender.keypair,
        listing.mint,
        lender.keypair.publicKey
      );

      await lender.program.methods
        .repossessCollateral()
        .accounts({
          escrowAccount: borrower.escrowAccount,
          lender: lender.keypair.publicKey,
          lenderTokenAccount: tokenAccount.address,
          listingAccount: borrower.listingAccount,
          mint: listing.mint,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      await borrower.program.methods
        .closeAccount()
        .accounts({
          borrower: borrower.keypair.publicKey,
          listingAccount: borrower.listingAccount,
        })
        .rpc();

      try {
        await borrower.program.account.listing.fetch(borrower.listingAccount);
      } catch (err) {
        assert.equal(
          err.message,
          `Account does not exist ${borrower.listingAccount.toBase58()}`
        );
      }
    });

    it("Will not allow a loan to be repossessed if not overdue", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 60 * 60, // 1 hour
      };
      const borrower = await helpers.initLoan(connection, options);

      const lender = await helpers.createLoan(connection, borrower);

      const listing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );

      const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
        connection,
        lender.keypair,
        listing.mint,
        lender.keypair.publicKey
      );

      try {
        await lender.program.methods
          .repossessCollateral()
          .accounts({
            escrowAccount: listing.escrow,
            lender: lender.keypair.publicKey,
            lenderTokenAccount: tokenAccount.address,
            listingAccount: borrower.listingAccount,
            mint: listing.mint,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: splToken.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();

        assert.ok(false);
      } catch (error) {
        assert.ok(error.toString(), "This loan is not overdue");
      }
    });

    it("Will only allow lender to repossess an overdue loan", async () => {
      const options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        basisPoints: 500,
        duration: 1, // 1 second
      };
      const borrower = await helpers.initLoan(connection, options);

      const lender = await helpers.createLoan(connection, borrower);

      await wait(1); // ensure 1 second passes

      const listing = await borrower.program.account.listing.fetch(
        borrower.listingAccount
      );

      // Creates another signer
      const keypair = anchor.web3.Keypair.generate();
      const provider = helpers.getProvider(connection, keypair);
      const program = helpers.getProgram(provider);
      await helpers.requestAirdrop(connection, keypair.publicKey);

      const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
        connection,
        keypair,
        listing.mint,
        keypair.publicKey
      );

      try {
        await program.methods
          .repossessCollateral()
          .accounts({
            escrowAccount: listing.escrow,
            lender: lender.keypair.publicKey,
            lenderTokenAccount: tokenAccount.address,
            listingAccount: borrower.listingAccount,
            mint: listing.mint,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: splToken.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();

        assert.ok(false);
      } catch (error) {
        assert.ok(
          error.toString().includes("Error: Signature verification failed")
        );
      }

      try {
        await program.methods
          .repossessCollateral()
          .accounts({
            escrowAccount: listing.escrow,
            lender: keypair.publicKey,
            lenderTokenAccount: tokenAccount.address,
            listingAccount: borrower.listingAccount,
            mint: listing.mint,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: splToken.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();

        assert.ok(false);
      } catch (error) {
        assert.ok(error.toString().includes("A raw constraint was violated"));
      }
    });
  });

  describe("Call Options", () => {
    it("Will allow a call option to be created", async () => {
      const options = {
        amount: 1_000_000,
        strikePrice: anchor.web3.LAMPORTS_PER_SOL,
        expiry: Math.round(Date.now() / 1000) + 30 * 24 * 60 * 2, // 2 days
      };
      const seller = await helpers.initCallOption(connection, options);

      const callOption = await seller.program.account.callOption.fetch(
        seller.callOptionAccount
      );
      const sellerTokenAccount = await splToken.getAccount(
        connection,
        seller.associatedAddress
      );
      const escrowTokenAccount = await splToken.getAccount(
        connection,
        seller.escrowAccount
      );

      assert.equal(sellerTokenAccount.delegate, callOption.escrow.toBase58());
      assert.equal(
        callOption.seller.toBase58(),
        seller.keypair.publicKey.toBase58()
      );
      assert.equal(callOption.strikePrice.toNumber(), options.strikePrice);
      assert.equal(callOption.expiry.toNumber(), options.expiry);
      assert.equal(callOption.mint.toBase58(), seller.mint.toBase58());
      assert.deepEqual(callOption.state, { listed: {} });
      assert.equal(sellerTokenAccount.amount, BigInt(1));
      assert.equal(escrowTokenAccount.amount, BigInt(0));
      assert.equal(
        escrowTokenAccount.owner.toBase58(),
        seller.escrowAccount.toBase58()
      );
    });

    it("Will allow a call option to be bought", async () => {
      const options = {
        amount: 1_000_000,
        strikePrice: anchor.web3.LAMPORTS_PER_SOL,
        expiry: Math.round(Date.now() / 1000) + 30 * 24 * 60 * 2, // 2 days
      };
      const seller = await helpers.initCallOption(connection, options);
      await helpers.buyCallOption(connection, seller);

      const callOption = await seller.program.account.callOption.fetch(
        seller.callOptionAccount
      );
      const sellerTokenAccount = await splToken.getAccount(
        connection,
        seller.associatedAddress
      );
      const escrowTokenAccount = await splToken.getAccount(
        connection,
        seller.escrowAccount
      );

      assert.equal(
        callOption.seller.toBase58(),
        seller.keypair.publicKey.toBase58()
      );
      assert.deepEqual(callOption.state, { active: {} });
      assert.equal(sellerTokenAccount.amount, BigInt(0));
      assert.equal(escrowTokenAccount.amount, BigInt(1));
    });

    it("Will allow a call option to be exercised", async () => {
      const options = {
        amount: 1_000_000,
        strikePrice: anchor.web3.LAMPORTS_PER_SOL,
        expiry: Math.round(Date.now() / 1000) + 30 * 24 * 60 * 2, // 2 days
      };
      const seller = await helpers.initCallOption(connection, options);
      const buyer = await helpers.buyCallOption(connection, seller);

      const beforeExerciseBalance = await connection.getBalance(
        buyer.keypair.publicKey
      );

      await buyer.program.methods
        .exerciseCallOption()
        .accounts({
          seller: seller.keypair.publicKey,
          buyer: buyer.keypair.publicKey,
          buyerTokenAccount: buyer.associatedAddress,
          callOptionAccount: seller.callOptionAccount,
          escrowAccount: seller.escrowAccount,
          mint: seller.mint,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: splToken.TOKEN_PROGRAM_ID,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      const afterExerciseBalance = await connection.getBalance(
        buyer.keypair.publicKey
      );
      const callOption = await seller.program.account.callOption.fetch(
        seller.callOptionAccount
      );
      const buyerTokenAccount = await splToken.getAccount(
        connection,
        buyer.associatedAddress
      );

      assert.equal(
        beforeExerciseBalance - anchor.web3.LAMPORTS_PER_SOL - 5000,
        afterExerciseBalance
      );
      assert.deepEqual(callOption.state, { exercised: {} });
      assert.equal(buyerTokenAccount.amount, BigInt(1));
    });

    it("Will NOT allow a call option to be exercised if expired", async () => {
      const options = {
        amount: 1_000_000,
        strikePrice: anchor.web3.LAMPORTS_PER_SOL,
        expiry: Math.round(Date.now() / 1000) + 2, // 2 seconds
      };
      const seller = await helpers.initCallOption(connection, options);
      const buyer = await helpers.buyCallOption(connection, seller);

      await wait(2);

      try {
        await buyer.program.methods
          .exerciseCallOption()
          .accounts({
            seller: seller.keypair.publicKey,
            buyer: buyer.keypair.publicKey,
            buyerTokenAccount: buyer.associatedAddress,
            callOptionAccount: seller.callOptionAccount,
            escrowAccount: seller.escrowAccount,
            mint: seller.mint,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: splToken.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .rpc();

        assert.fail("Expected error");
      } catch (error) {
        assert.ok(error.message.includes("Option expired"));
      }
    });

    it("Will allow a call option to be closed if not active", async () => {
      const options = {
        amount: 1_000_000,
        strikePrice: anchor.web3.LAMPORTS_PER_SOL,
        expiry: Math.round(Date.now() / 1000) + 30 * 24 * 60 * 2, // 2 days
      };
      const seller = await helpers.initCallOption(connection, options);

      await seller.program.methods
        .closeCallOption()
        .accounts({
          depositTokenAccount: seller.associatedAddress,
          callOptionAccount: seller.callOptionAccount,
          escrowAccount: seller.escrowAccount,
          mint: seller.mint,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        })
        .rpc();

      try {
        await seller.program.account.callOption.fetch(seller.callOptionAccount);
        assert.fail();
      } catch (error) {
        assert.ok(error.message.includes("Account does not exist"));
      }
      const sellerTokenAccount = await splToken.getAccount(
        connection,
        seller.associatedAddress
      );

      assert.equal(sellerTokenAccount.amount, BigInt(1));
      assert.equal(sellerTokenAccount.delegate, null);
    });

    it("Will allow a call option to be closed if expired", async () => {
      const options = {
        amount: 1_000_000,
        strikePrice: anchor.web3.LAMPORTS_PER_SOL,
        expiry: Math.round(Date.now() / 1000) + 2, // 2 seconds
      };
      const seller = await helpers.initCallOption(connection, options);
      await helpers.buyCallOption(connection, seller);

      await wait(2);

      await seller.program.methods
        .closeCallOption()
        .accounts({
          depositTokenAccount: seller.associatedAddress,
          callOptionAccount: seller.callOptionAccount,
          escrowAccount: seller.escrowAccount,
          mint: seller.mint,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        })
        .rpc();

      try {
        await seller.program.account.callOption.fetch(seller.callOptionAccount);
        assert.fail();
      } catch (error) {
        assert.ok(error.message.includes("Account does not exist"));
      }
      const sellerTokenAccount = await splToken.getAccount(
        connection,
        seller.associatedAddress
      );

      assert.equal(sellerTokenAccount.amount, BigInt(1));
      assert.equal(sellerTokenAccount.delegate, null);
    });
  });
});

async function wait(seconds) {
  await new Promise((resolve) => setTimeout(resolve, seconds * 1000));
}
