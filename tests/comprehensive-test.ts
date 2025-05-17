import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { PredictionMarketClient } from "./contracts";

async function testForesightProtocol() {
  console.log("Starting comprehensive Foresight Protocol test...");
  
  const connection = new anchor.web3.Connection("http://localhost:8899", "confirmed");
  const admin = Keypair.generate();
  const creator = Keypair.generate();
  const user1 = Keypair.generate();
  const user2 = Keypair.generate();
  
  console.log("Admin public key:", admin.publicKey.toString());
  console.log("Creator public key:", creator.publicKey.toString());
  console.log("User1 public key:", user1.publicKey.toString());
  console.log("User2 public key:", user2.publicKey.toString());
  
  console.log("\nFunding accounts with SOL...");
  await connection.requestAirdrop(admin.publicKey, 5 * LAMPORTS_PER_SOL);
  await connection.requestAirdrop(creator.publicKey, 5 * LAMPORTS_PER_SOL);
  await connection.requestAirdrop(user1.publicKey, 5 * LAMPORTS_PER_SOL);
  await connection.requestAirdrop(user2.publicKey, 5 * LAMPORTS_PER_SOL);
  
  await new Promise(resolve => setTimeout(resolve, 2000));
  
  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(admin),
    { commitment: "confirmed" }
  );
  const client = new PredictionMarketClient(provider);
  console.log("Connected to program with ID:", client.program.programId.toString());
  
  try {
    console.log("\nStep 1: Initialize Protocol Stats");
    try {
      const txInitProtocolStats = await client.initializeProtocolStats(admin);
      console.log("Protocol stats initialized:", txInitProtocolStats);
    } catch (error) {
      if (error.message.includes("already in use")) {
        console.log("Protocol stats already initialized, continuing...");
      } else {
        throw error;
      }
    }
    
    const [protocolStatsAddress] = await client.findProtocolStatsAddress();
    const protocolStats = await client.program.account.protocolStats.fetch(protocolStatsAddress);
    console.log("Protocol stats data:", {
      totalMarkets: protocolStats.totalMarkets,
      totalUsers: protocolStats.totalUsers,
      totalVolume: protocolStats.totalVolume.toString(),
    });

    console.log("\nStep 2: Create Creator Profile");
    try {
      const txCreateCreatorProfile = await client.createCreatorProfile(creator);
      console.log("Creator profile created:", txCreateCreatorProfile);
    } catch (error) {
      if (error.message.includes("already in use")) {
        console.log("Creator profile already exists, continuing...");
      } else {
        throw error;
      }
    }
    
    console.log("\nStep 3: Initialize AI Resolver");
    try {
      const txInitAiResolver = await client.initializeAiResolver(admin);
      console.log("AI resolver initialized:", txInitAiResolver);
    } catch (error) {
      if (error.message.includes("already in use")) {
        console.log("AI resolver already initialized, continuing...");
      } else {
        throw error;
      }
    }

    console.log("\nStep 4: Create Token Mint");
    const mint = await createMint(
      connection,
      admin,
      admin.publicKey,
      admin.publicKey,
      6
    );
    console.log("Mint created:", mint.toString());

    const creatorAta = await getOrCreateAssociatedTokenAccount(
      connection,
      creator,
      mint,
      creator.publicKey
    );
    const user1Ata = await getOrCreateAssociatedTokenAccount(
      connection,
      user1,
      mint,
      user1.publicKey
    );
    const user2Ata = await getOrCreateAssociatedTokenAccount(
      connection,
      user2,
      mint,
      user2.publicKey
    );

    await mintTo(
      connection,
      admin,
      mint,
      creatorAta.address,
      admin.publicKey,
      1000_000_000 
    );
    await mintTo(
      connection,
      admin,
      mint,
      user1Ata.address,
      admin.publicKey,
      1000_000_000 
    );
    await mintTo(
      connection,
      admin,
      mint,
      user2Ata.address,
      admin.publicKey,
      1000_000_000 
    );
    console.log("Tokens minted to all users");
    

    console.log("\nStep 5: Create Market");
    const question = "Will BTC exceed $100k by the end of 2025?";
    const outcomes = ["Yes", "No"];
    const aiScore = 90;
    const resolutionTime = new anchor.BN(Math.floor(Date.now() / 1000) + 15552000);
    const aiClassification = 1;
    const creatorMetadata = "Test market created via comprehensive test";
  
    const txCreateMarket = await client.createMarket(
      creator,
      mint,
      question,
      outcomes,
      aiScore,
      resolutionTime,
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
      totalPool: marketData.totalPool.toString(),
      creatorFeeBps: marketData.creatorFeeBps.toString(),
    });
    
    console.log("\nStep 6: Create User Profiles");
    try {
      const txCreateUser1Profile = await client.initializeUserProfile(user1);
      console.log("User1 profile created:", txCreateUser1Profile);
    } catch (error) {
      if (error.message.includes("already in use")) {
        console.log("User1 profile already exists, continuing...");
      } else {
        throw error;
      }
    }
    
    try {
      const txCreateUser2Profile = await client.initializeUserProfile(user2);
      console.log("User2 profile created:", txCreateUser2Profile);
    } catch (error) {
      if (error.message.includes("already in use")) {
        console.log("User2 profile already exists, continuing...");
      } else {
        throw error;
      }
    }
    
    console.log("\nStep 7: Stake in Market");
    
    const [marketVaultAddress] = await client.findMarketVaultAddress(marketAddress);
    
    const stake1Amount = new anchor.BN(100_000_000);
    const txStake1 = await client.stakePrediction(
      user1,
      marketAddress,
      user1Ata.address,
      0,
      stake1Amount
    );
    console.log("User1 staked on YES, transaction signature:", txStake1);
    
    const stake2Amount = new anchor.BN(50_000_000);
    const txStake2 = await client.stakePrediction(
      user2,
      marketAddress,
      user2Ata.address,
      1,
      stake2Amount
    );
    console.log("User2 staked on NO, transaction signature:", txStake2);
    
    console.log("\nStep 8: Verify Market Data After Staking");
    const marketDataAfterStake = await client.program.account.market.fetch(marketAddress);
    console.log("Market data after staking:", {
      totalPool: marketDataAfterStake.totalPool.toString(),
      stakesPerOutcome: marketDataAfterStake.stakesPerOutcome.map(x => x.toString()),
    });
    
    console.log("\nStep 9: Resolve Market with AI");
    const winningOutcomeIndex = 0;
    const txResolveWithAi = await client.resolveMarketViaAi(
      admin,
      marketAddress,
      winningOutcomeIndex,
      0.95,
      "AI has determined BTC will exceed $100k by end of 2025"
    );
    console.log("Market resolved with AI, transaction signature:", txResolveWithAi);
    
    console.log("\nStep 10: Claim Winnings");
    
    const protocolFeeAddress = creatorAta.address;
    
    const txClaimWinnings = await client.claimReward(
      user1,
      marketAddress,
      user1Ata.address,
      creatorAta.address,
      protocolFeeAddress
    );
    console.log("User1 claimed winnings, transaction signature:", txClaimWinnings);
    
    console.log("\nStep 11: Verify Final Token Balances");
    const finalCreatorAta = await connection.getTokenAccountBalance(creatorAta.address);
    const finalUser1Ata = await connection.getTokenAccountBalance(user1Ata.address);
    const finalUser2Ata = await connection.getTokenAccountBalance(user2Ata.address);
    
    console.log("Final token balances:");
    console.log("- Creator:", finalCreatorAta.value.uiAmount);
    console.log("- User1 (winner):", finalUser1Ata.value.uiAmount);
    console.log("- User2 (loser):", finalUser2Ata.value.uiAmount);
    
    console.log("\nStep 12: Check Updated Protocol Stats");
    const updatedProtocolStats = await client.program.account.protocolStats.fetch(protocolStatsAddress);
    console.log("Updated protocol stats:", {
      totalMarkets: updatedProtocolStats.totalMarkets,
      totalUsers: updatedProtocolStats.totalUsers,
      totalStakes: updatedProtocolStats.totalStakes.toString(),
      totalVolume: updatedProtocolStats.totalVolume.toString(),
    });
    
    console.log("\nAll tests completed successfully!");
    
  } catch (error) {
    console.error("Error during testing:", error);
    console.error("Error stack:", error.stack);
  }
}

testForesightProtocol().catch(console.error);