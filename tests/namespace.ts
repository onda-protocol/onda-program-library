import assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as helpers from "./helpers";

describe.only("Namespace", () => {
  it("creates a namespace", async () => {
    const name = "test";
    const uri = "https://test.com";
    const admin = anchor.web3.Keypair.generate();
    const merkleTree = anchor.web3.Keypair.generate();
    const namespaceProgram = await helpers.getNamespaceProgram(admin);
    const namespacePda = await helpers.findNamespacePda(name);
    const treeMarkerPda = await helpers.findTreeMarkerPda(merkleTree.publicKey);
    const forumConfigPda = await helpers.findForumConfigPda(
      merkleTree.publicKey
    );

    await helpers.requestAirdrop(admin.publicKey);
    await helpers.initForum(admin, merkleTree);

    await namespaceProgram.methods
      .createNamespace(name, uri)
      .accounts({
        admin: admin.publicKey,
        payer: admin.publicKey,
        namespace: namespacePda,
        treeMarker: treeMarkerPda,
        forumConfig: forumConfigPda,
        merkleTree: merkleTree.publicKey,
      })
      .rpc({
        skipPreflight: true,
      });
  });
});
