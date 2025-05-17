import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { PredictionMarketClient } from "./contracts";

async function testForesightProtocol() {
  const connection = new anchor.web3.Connection("http://localhost:8899", "confirmed");
  const admin = Keypair.generate();
  const creator = Keypair.generate();
  const user = Keypair.generate();
  
  console.log("Admin public key:", admin.publicKey.toString());
  console.log("Creator public key:", creator.publicKey.toString());
  console.log("User public key:", user.publicKey.toString());
  
  console.log("Requesting airdrop for admin...");
  await connection.requestAirdrop(admin.publicKey, 2 * LAMPORTS_PER_SOL);
  console.log("Requesting airdrop for creator...");
  await connection.requestAirdrop(creator.publicKey, 2 * LAMPORTS_PER_SOL);
  console.log("Requesting airdrop for user...");
  await connection.requestAirdrop(user.publicKey, 2 * LAMPORTS_PER_SOL);
  
  await new Promise(resolve => setTimeout(resolve, 2000));
  
  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(admin),
    { commitment: "confirmed" }
  );
  const client = new PredictionMarketClient(provider);
  console.log("Connected to program with ID:", client.program.programId.toString());
  
  try {
    console.log("\n--- Step 1: Initialize Protocol Stats ---");
    const txInitProtocolStats = await client.initializeProtocolStats(admin);
    console.log("Transaction signature:", txInitProtocolStats);
    
    const [protocolStatsAddress] = await client.findProtocolStatsAddress();
    const protocolStats = await client.program.account.protocolStats.fetch(protocolStatsAddress);
    console.log("Protocol stats initialized:", protocolStats);
    
    console.log("\n--- Step 2: Create Creator Profile ---");
    const txCreateCreatorProfile = await client.createCreatorProfile(creator);
    console.log("Transaction signature:", txCreateCreatorProfile);
    
    console.log("\n--- Step 3: Initialize AI Resolver ---");
    const txInitAiResolver = await client.initializeAiResolver(admin);
    console.log("Transaction signature:", txInitAiResolver);
    
    console.log("\n--- Step 4: Create Token Mint ---");
    const mint = await createMint(
      connection,
      admin,
      admin.publicKey,
      admin.publicKey,
      6
    );
    console.log("Mint created:", mint.toString());
    
    console.log("\n--- Step 5: Create Market ---");
    const question = "Will ETH hit $5,000 by end of 2025?";
    const outcomes = ["Yes", "No"];
    const aiScore = 80;
    const aiRecommendedResolutionTime = new anchor.BN(Math.floor(Date.now() / 1000) + 15552000);
    const aiClassification = 1;
    const creatorMetadata = "Test market by Foresight Protocol";
    
    const txCreateMarket = await client.createMarket(
      creator,
      mint,
      question,
      outcomes,
      aiScore,
      aiRecommendedResolutionTime,
      aiClassification,
      creatorMetadata,
      undefined,
      true
    );
    console.log("Market created, transaction signature:", txCreateMarket);
    
    const [creatorProfileAddress] = await client.findCreatorProfileAddress(creator.publicKey);
    const creatorProfile = await client.program.account.creatorProfile.fetch(creatorProfileAddress);
    
    const marketIndex = creatorProfile.marketsCreated - 1;
    console.log("Market index:", marketIndex);
    
    const [marketAddress] = await client.findMarketAddress(creator.publicKey, marketIndex);
    console.log("Market address:", marketAddress.toString());
    
    const marketData = await client.program.account.market.fetch(marketAddress);
    console.log("Market data:", {
      question: marketData.question,
      outcomes: marketData.outcomes,
      creator: marketData.creator.toString(),
      creatorFeeBps: marketData.creatorFeeBps.toString()
    });
    
    console.log("\nAll tests completed successfully!");
    
  } catch (error) {
    console.error("Error during testing:", error);
  }
}

testForesightProtocol().catch(console.error);