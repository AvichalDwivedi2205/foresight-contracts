import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { PredictionMarketClient } from "./contracts";

async function testForesightProtocol() {
  // Set up connection to local validator
  const connection = new anchor.web3.Connection("http://localhost:8899", "confirmed");
  const admin = Keypair.generate();
  const creator = Keypair.generate();
  const user = Keypair.generate();
  
  console.log("Admin public key:", admin.publicKey.toString());
  console.log("Creator public key:", creator.publicKey.toString());
  console.log("User public key:", user.publicKey.toString());
  
  // Fund accounts with SOL
  console.log("Requesting airdrop for admin...");
  await connection.requestAirdrop(admin.publicKey, 2 * LAMPORTS_PER_SOL);
  console.log("Requesting airdrop for creator...");
  await connection.requestAirdrop(creator.publicKey, 2 * LAMPORTS_PER_SOL);
  console.log("Requesting airdrop for user...");
  await connection.requestAirdrop(user.publicKey, 2 * LAMPORTS_PER_SOL);
  
  // Wait for confirmation
  await new Promise(resolve => setTimeout(resolve, 2000));
  
  // Create provider and client
  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(admin),
    { commitment: "confirmed" }
  );
  const client = new PredictionMarketClient(provider);
  console.log("Connected to program with ID:", client.program.programId.toString());
  
  try {
    // 1. Initialize Protocol Stats
    console.log("\n--- Step 1: Initialize Protocol Stats ---");
    const txInitProtocolStats = await client.initializeProtocolStats(admin);
    console.log("Transaction signature:", txInitProtocolStats);
    
    // Fetch the initialized protocol stats
    const [protocolStatsAddress] = await client.findProtocolStatsAddress();
    const protocolStats = await client.program.account.protocolStats.fetch(protocolStatsAddress);
    console.log("Protocol stats initialized:", protocolStats);
    
    // 2. Create Creator Profile
    console.log("\n--- Step 2: Create Creator Profile ---");
    const txCreateCreatorProfile = await client.createCreatorProfile(creator);
    console.log("Transaction signature:", txCreateCreatorProfile);
    
    // 3. Initialize AI Resolver
    console.log("\n--- Step 3: Initialize AI Resolver ---");
    const txInitAiResolver = await client.initializeAiResolver(admin);
    console.log("Transaction signature:", txInitAiResolver);
    
    // 4. Create a test token mint
    console.log("\n--- Step 4: Create Token Mint ---");
    const mint = await createMint(
      connection,
      admin,
      admin.publicKey,
      admin.publicKey,
      6 // decimals
    );
    console.log("Mint created:", mint.toString());
    
    // 5. Create a market
    console.log("\n--- Step 5: Create Market ---");
    const question = "Will ETH hit $5,000 by end of 2025?";
    const outcomes = ["Yes", "No"];
    const aiScore = 80; // 0-100
    // 6 months from now in seconds
    const aiRecommendedResolutionTime = new anchor.BN(Math.floor(Date.now() / 1000) + 15552000);
    const aiClassification = 1; // Arbitrary category
    const creatorMetadata = "Test market by Foresight Protocol";
    
    // Using tier-based fee system instead of creator-specified fee
    const txCreateMarket = await client.createMarket(
      creator,
      mint,
      question,
      outcomes,
      aiScore,
      aiRecommendedResolutionTime,
      aiClassification,
      creatorMetadata,
      undefined, // Fee is now determined by creator's tier (Tier 0 = 1.5%)
      true // AI resolvable
    );
    console.log("Market created, transaction signature:", txCreateMarket);
    
    // Find the market address
    const [creatorProfileAddress] = await client.findCreatorProfileAddress(creator.publicKey);
    const creatorProfile = await client.program.account.creatorProfile.fetch(creatorProfileAddress);
    
    // Get market index from creator profile
    const marketIndex = creatorProfile.marketsCreated - 1;
    console.log("Market index:", marketIndex);
    
    const [marketAddress] = await client.findMarketAddress(creator.publicKey, marketIndex);
    console.log("Market address:", marketAddress.toString());
    
    // Optional: Fetch market data to verify
    const marketData = await client.program.account.market.fetch(marketAddress);
    console.log("Market data:", {
      question: marketData.question,
      outcomes: marketData.outcomes,
      creator: marketData.creator.toString(),
      creatorFeeBps: marketData.creatorFeeBps.toString() // Should be 150 (1.5%) for tier 0
    });
    
    console.log("\n🎉 All tests completed successfully!");
    
  } catch (error) {
    console.error("Error during testing:", error);
  }
}

testForesightProtocol().catch(console.error);