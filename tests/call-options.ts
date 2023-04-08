require("dotenv").config();

import assert from "assert";
import {
  Metadata,
  PROGRAM_ID as METADATA_PROGRAM_ID,
} from "@metaplex-foundation/mpl-token-metadata";
import { PROGRAM_ID as AUTHORIZATION_RULES_PROGRAM_ID } from "@metaplex-foundation/mpl-token-auth-rules";
import * as anchor from "@project-serum/anchor";
import * as splToken from "@solana/spl-token";
import * as helpers from "./helpers";

// Configure the client to use the local cluster.
const connection = new anchor.web3.Connection(
  "http://127.0.0.1:8899",
  anchor.AnchorProvider.defaultOptions().preflightCommitment
);

describe("Call Options", () => {
  describe("Bids", () => {
    let buyer: helpers.CallOptionBidBuyer;
    let seller: helpers.CallOptionBidSeller;
    let options;

    it("Creates a call option bid", async () => {
      options = {
        amount: anchor.web3.LAMPORTS_PER_SOL,
        strikePrice: 500,
        expiry: Date.now() / 1000 + 86_400,
      };

      buyer = await helpers.bidCallOption(connection, options);

      const bid = await buyer.program.account.callOptionBid.fetch(
        buyer.callOptionBid
      );
      assert.equal(bid.amount.toNumber(), options.amount);
    });
  });

  describe("Exercise call option", () => {
    let options;
    let seller: helpers.CallOptionSeller;
    let buyer: helpers.CallOptionBuyer;

    it("Creates a call option ask", async () => {
      options = {
        amount: 1_000_000,
        strikePrice: anchor.web3.LAMPORTS_PER_SOL,
        expiry: Math.round(Date.now() / 1000) + 30 * 24 * 60 * 2, // 2 days
      };
      seller = await helpers.askCallOption(connection, options);

      const callOption = await seller.program.account.callOption.fetch(
        seller.callOption
      );
      const tokenManager = await seller.program.account.tokenManager.fetch(
        seller.tokenManager
      );
      const sellerTokenAccount = await splToken.getAccount(
        connection,
        seller.depositTokenAccount
      );

      assert.deepEqual(tokenManager.accounts, {
        rental: false,
        callOption: true,
        loan: false,
      });
      assert.equal(
        sellerTokenAccount.delegate.toBase58(),
        seller.tokenManager.toBase58()
      );
      assert.equal(
        callOption.seller.toBase58(),
        seller.keypair.publicKey.toBase58()
      );
      assert.equal(callOption.strikePrice.toNumber(), options.strikePrice);
      assert.equal(callOption.expiry.toNumber(), options.expiry);
      assert.equal(callOption.mint.toBase58(), seller.mint.toBase58());
      assert.deepEqual(callOption.state, { listed: {} });
      assert.equal(sellerTokenAccount.amount, BigInt(1));
    });

    it("Freezes tokens after initialization", async () => {
      const tokenAccount = await splToken.getAccount(
        connection,
        seller.depositTokenAccount
      );
      assert.equal(tokenAccount.isFrozen, true);
    });

    it("Buys a call option", async () => {
      buyer = await helpers.buyCallOption(connection, seller);

      const callOption = await seller.program.account.callOption.fetch(
        seller.callOption
      );
      assert.equal(
        callOption.seller.toBase58(),
        seller.keypair.publicKey.toBase58()
      );
      assert.deepEqual(callOption.state, { active: {} });
    });

    it("Can't be closed if active", async () => {
      const signer = await helpers.getSigner();

      try {
        await seller.program.methods
          .closeCallOption()
          .accounts({
            signer: signer.publicKey,
            callOption: seller.callOption,
            tokenManager: seller.tokenManager,
            seller: seller.keypair.publicKey,
            depositTokenAccount: seller.depositTokenAccount,
            depositTokenRecord: seller.tokenRecord,
            mint: seller.mint,
            metadata: seller.metadata,
            edition: seller.edition,
            authorizationRules: null,
            authorizationRulesProgram: AUTHORIZATION_RULES_PROGRAM_ID,
            metadataProgram: METADATA_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
            sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
            tokenProgram: splToken.TOKEN_PROGRAM_ID,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          })
          .signers([signer])
          .rpc();
        assert.fail("Active call option was closed!");
      } catch (err) {
        assert(err instanceof anchor.AnchorError);
        assert.equal(err.error.errorCode.code, "OptionNotExpired");
      }
    });

    it("Exercises a call option", async () => {
      const signer = await helpers.getSigner();
      const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
        connection,
        buyer.keypair,
        seller.mint,
        buyer.keypair.publicKey
      );
      const metadata = await Metadata.fromAccountAddress(
        connection,
        seller.metadata
      );
      const beforeBuyerBalance = await connection.getBalance(
        buyer.keypair.publicKey
      );
      const beforeSellerBalance = await connection.getBalance(
        seller.keypair.publicKey
      );
      const escrowTokenAccount = helpers.findEscrowTokenAccount(
        seller.tokenManager
      );
      const escrowTokenRecord = helpers.findTokenRecordAddress(
        seller.mint,
        escrowTokenAccount
      );

      let txFee;

      const accounts = {
        escrowTokenAccount,
        escrowTokenRecord,
        signer: signer.publicKey,
        seller: seller.keypair.publicKey,
        buyer: buyer.keypair.publicKey,
        callOption: seller.callOption,
        tokenManager: seller.tokenManager,
        buyerTokenAccount: tokenAccount.address,
        depositTokenAccount: seller.depositTokenAccount,
        depositTokenRecord: seller.tokenRecord,
        mint: seller.mint,
        metadata: seller.metadata,
        edition: seller.edition,
        authorizationRules: null,
        authorizationRulesProgram: AUTHORIZATION_RULES_PROGRAM_ID,
        metadataProgram: METADATA_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        sysvarInstructions: anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenProgram: splToken.TOKEN_PROGRAM_ID,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      };

      try {
        const exerciseIx = await buyer.program.methods
          .exerciseCallOption()
          .accounts(accounts)
          .remainingAccounts(
            metadata.data.creators.map((creator) => ({
              pubkey: creator.address,
              isSigner: false,
              isWritable: true,
            }))
          )
          .instruction();

        const instructions = [
          anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
            units: 210_000,
          }),
          exerciseIx,
        ];

        const latestBlockhash = await connection.getLatestBlockhash();
        const messageV0 = new anchor.web3.TransactionMessage({
          payerKey: buyer.keypair.publicKey,
          recentBlockhash: latestBlockhash.blockhash,
          instructions,
        }).compileToV0Message();
        const transaction = new anchor.web3.VersionedTransaction(messageV0);
        transaction.sign([signer, buyer.keypair]);
        const signature = await connection.sendTransaction(transaction);
        await connection.confirmTransaction(
          {
            signature,
            ...latestBlockhash,
          },
          "confirmed"
        );
        const tx = await connection.getTransaction(signature, {
          commitment: "confirmed",
          maxSupportedTransactionVersion: 0,
        });
        txFee = tx.meta.fee;
      } catch (err) {
        console.log(err.logs);
        assert.fail(err);
      }

      const afterBuyerBalance = await connection.getBalance(
        buyer.keypair.publicKey
      );
      const afterSellerBalance = await connection.getBalance(
        seller.keypair.publicKey
      );
      const escrowTokenAccountRent = await connection.getBalance(
        escrowTokenAccount
      );
      const escrowTokenRecordRent = await connection.getBalance(
        escrowTokenRecord
      );
      const callOption = await seller.program.account.callOption.fetch(
        seller.callOption
      );
      const tokenManager = await seller.program.account.tokenManager.fetch(
        seller.tokenManager
      );
      const buyerTokenAccount = await splToken.getAccount(
        connection,
        escrowTokenAccount
      );

      const creatorFees =
        (metadata.data.sellerFeeBasisPoints / 10_000) *
        callOption.strikePrice.toNumber();
      // This can be reclaimed by the buyer when claiming
      const additionalRent = escrowTokenAccountRent + escrowTokenRecordRent;

      const estimatedBuyerBalance =
        beforeBuyerBalance - options.strikePrice - txFee - additionalRent;

      const estimatedSellerBalance =
        beforeSellerBalance + (options.strikePrice - creatorFees);

      assert.equal(estimatedBuyerBalance, afterBuyerBalance, "buyer balance");
      assert.equal(
        estimatedSellerBalance,
        afterSellerBalance,
        "seller balance"
      );
      assert.deepEqual(callOption.state, { exercised: {} });
      assert.equal(
        tokenManager.authority.toBase58(),
        buyer.keypair.publicKey.toBase58()
      );
      assert.equal(buyerTokenAccount.amount, BigInt(1));
    });

    //     it("Can be closed after being exercised", async () => {
    //       const signer = await helpers.getSigner();

    //       await seller.program.methods
    //         .closeCallOption()
    //         .accounts({
    //           signer: signer.publicKey,
    //           seller: seller.keypair.publicKey,
    //           callOption: seller.callOption,
    //           tokenManager: seller.tokenManager,
    //           depositTokenAccount: seller.depositTokenAccount,
    //           mint: seller.mint,
    //           edition: seller.edition,
    //           metadataProgram: METADATA_PROGRAM_ID,
    //           systemProgram: anchor.web3.SystemProgram.programId,
    //           tokenProgram: splToken.TOKEN_PROGRAM_ID,
    //           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
    //         })
    //         .signers([signer])
    //         .rpc();

    //       try {
    //         await seller.program.account.callOption.fetch(seller.callOption);
    //         assert.fail();
    //       } catch (error) {
    //         assert.ok(error.message.includes("Account does not exist"));
    //       }
    //       const sellerTokenAccount = await splToken.getAccount(
    //         connection,
    //         seller.depositTokenAccount
    //       );

    //       assert.equal(sellerTokenAccount.amount, BigInt(0));
    //       assert.equal(sellerTokenAccount.delegate, null);
    //     });
  });

  //   describe("Call option expiry", () => {
  //     let options;
  //     let seller: Awaited<ReturnType<typeof helpers.askCallOption>>;
  //     let buyer: Awaited<ReturnType<typeof helpers.buyCallOption>>;

  //     it("Creates a dexloan call option", async () => {
  //       options = {
  //         amount: 1_000_000,
  //         strikePrice: anchor.web3.LAMPORTS_PER_SOL,
  //         expiry: Math.round(Date.now() / 1000) + 20, // 20 seconds
  //       };
  //       seller = await helpers.askCallOption(connection, options);

  //       const callOption = await seller.program.account.callOption.fetch(
  //         seller.callOption
  //       );
  //       const sellerTokenAccount = await splToken.getAccount(
  //         connection,
  //         seller.depositTokenAccount
  //       );

  //       assert.equal(
  //         sellerTokenAccount.delegate.toBase58(),
  //         seller.tokenManager.toBase58()
  //       );
  //       assert.equal(
  //         callOption.seller.toBase58(),
  //         seller.keypair.publicKey.toBase58()
  //       );
  //       assert.equal(callOption.strikePrice.toNumber(), options.strikePrice);
  //       assert.equal(callOption.expiry.toNumber(), options.expiry);
  //       assert.equal(callOption.mint.toBase58(), seller.mint.toBase58());
  //       assert.deepEqual(callOption.state, { listed: {} });
  //       assert.equal(sellerTokenAccount.amount, BigInt(1));
  //     });

  //     it("Buys a call option", async () => {
  //       const sellerBeforeBalance = await connection.getBalance(
  //         seller.keypair.publicKey
  //       );

  //       buyer = await helpers.buyCallOption(connection, seller);

  //       const callOption = await seller.program.account.callOption.fetch(
  //         seller.callOption
  //       );
  //       const sellerAfterBalance = await connection.getBalance(
  //         seller.keypair.publicKey
  //       );
  //       const estimatedSellerBalance =
  //         sellerBeforeBalance + options.amount - options.amount * 0.02;

  //       assert.equal(
  //         sellerAfterBalance,
  //         estimatedSellerBalance,
  //         "seller balance"
  //       );
  //       assert.equal(
  //         callOption.seller.toBase58(),
  //         seller.keypair.publicKey.toBase58()
  //       );
  //       assert.deepEqual(callOption.state, { active: {} });
  //     });

  //     it("Cannot be exercised if expired", async () => {
  //       const signer = await helpers.getSigner();
  //       const callOption = await seller.program.account.callOption.fetch(
  //         seller.callOption
  //       );
  //       const now = Date.now() / 1000;
  //       const timeUntilExpiry = Math.ceil(callOption.expiry.toNumber() - now);
  //       await helpers.wait(timeUntilExpiry + 1);

  //       try {
  //         const tokenAccount = await splToken.getOrCreateAssociatedTokenAccount(
  //           connection,
  //           buyer.keypair,
  //           seller.mint,
  //           buyer.keypair.publicKey
  //         );

  //         await buyer.program.methods
  //           .exerciseCallOption()
  //           .accounts({
  //             signer: signer.publicKey,
  //             seller: seller.keypair.publicKey,
  //             buyer: buyer.keypair.publicKey,
  //             buyerTokenAccount: tokenAccount.address,
  //             callOption: seller.callOption,
  //             tokenManager: seller.tokenManager,
  //             mint: seller.mint,
  //             edition: seller.edition,
  //             metadataProgram: METADATA_PROGRAM_ID,
  //             systemProgram: anchor.web3.SystemProgram.programId,
  //             tokenProgram: splToken.TOKEN_PROGRAM_ID,
  //             clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
  //             rent: anchor.web3.SYSVAR_RENT_PUBKEY,
  //           })
  //           .signers([signer])
  //           .rpc();
  //         assert.fail();
  //       } catch (error) {
  //         console.log(error.logs);
  //         assert.ok(true);
  //       }
  //     });

  //     it("Can be closed by seller when expired", async () => {
  //       const signer = await helpers.getSigner();

  //       await seller.program.methods
  //         .closeCallOption()
  //         .accounts({
  //           signer: signer.publicKey,
  //           seller: seller.keypair.publicKey,
  //           callOption: seller.callOption,
  //           tokenManager: seller.tokenManager,
  //           depositTokenAccount: seller.depositTokenAccount,
  //           mint: seller.mint,
  //           edition: seller.edition,
  //           metadataProgram: METADATA_PROGRAM_ID,
  //           systemProgram: anchor.web3.SystemProgram.programId,
  //           tokenProgram: splToken.TOKEN_PROGRAM_ID,
  //           clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
  //         })
  //         .signers([signer])
  //         .rpc();

  //       try {
  //         await seller.program.account.callOption.fetch(seller.callOption);
  //         assert.fail();
  //       } catch (error) {
  //         assert.ok(error.message.includes("Account does not exist"));
  //       }

  //       try {
  //         await seller.program.account.tokenManager.fetch(seller.tokenManager);
  //       } catch (error) {
  //         assert.ok(error.message.includes("Account does not exist"));
  //       }

  //       const sellerTokenAccount = await splToken.getAccount(
  //         connection,
  //         seller.depositTokenAccount
  //       );
  //       assert.equal(sellerTokenAccount.amount, BigInt(1));
  //       assert.equal(sellerTokenAccount.delegate, null);
  //     });
  //   });
});
