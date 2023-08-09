import * as anchor from "@project-serum/anchor";
import {
  Metaplex,
  keypairIdentity,
  NftWithToken,
} from "@metaplex-foundation/js";
import assert from "assert";
import { OndaProfile, IDL } from "../target/types/onda_profile";
import { requestAirdrop } from "./helpers";

const program = anchor.workspace.OndaProfile as anchor.Program<OndaProfile>;
const connection = program.provider.connection;

function findProfilePda(author: anchor.web3.PublicKey) {
  return anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("profile"), author.toBuffer()],
    program.programId
  )[0];
}

async function createAnchorProgram(
  keypair: anchor.web3.Keypair = anchor.web3.Keypair.generate()
) {
  await requestAirdrop(keypair.publicKey);
  return new anchor.Program<OndaProfile>(
    IDL,
    program.programId,
    new anchor.AnchorProvider(
      connection,
      new anchor.Wallet(keypair),
      anchor.AnchorProvider.defaultOptions()
    )
  );
}

describe("Profile", () => {
  let nft: NftWithToken;
  let user = anchor.web3.Keypair.generate();
  const profilePda = findProfilePda(user.publicKey);

  it("Updates a profile", async () => {
    const program = await createAnchorProgram(user);
    const metaplex = Metaplex.make(connection).use(keypairIdentity(user));
    const result = await metaplex.nfts().create({
      uri: "https://arweave.net/123",
      name: "My NFT",
      sellerFeeBasisPoints: 500,
      creators: [
        {
          address: user.publicKey,
          share: 100,
        },
      ],
    });
    nft = result.nft;

    const accounts = {
      author: user.publicKey,
      profile: profilePda,
      mint: nft.mint.address,
      metadata: nft.metadataAddress,
      tokenAccount: nft.token.address,
    };
    await program.methods.updateProfile("MrGM").accounts(accounts).rpc();

    const profile = await program.account.profile.fetch(profilePda);
    assert.equal(profile.name.replace(/\0/g, ""), "MrGM");
    assert.equal(profile.mint.toBase58(), nft.mint.address.toBase58());
  });

  it("Permissionlessly verifies a profile mint", async () => {
    const accounts = {
      author: user.publicKey,
      profile: profilePda,
      mint: nft.mint.address,
      metadata: nft.metadataAddress,
      tokenAccount: nft.token.address,
    };
    await program.methods.verifyProfile().accounts(accounts).rpc();
    assert.ok(true);
  });
});
