import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { PredictionMarketClient } from "./contracts";
import * as fs from 'fs';
import * as bs58 from 'bs58';


// Configure market distributions
const MARKET_DISTRIBUTION = {
  timebound: 10,
  openended: 5,
  total: 15
};

// Participant distribution - Each participant can be both a creator and a user
const PARTICIPANTS_COUNT = 5;
const MARKETS_PER_PARTICIPANT = 3; // Each participant creates 3 markets

// Market Questions and Outcomes
const MARKET_QUESTIONS = [
  "Will Bitcoin exceed $100,000 by the end of 2024?",
  "Will Ethereum transition to PoS before July 2023?",
  "Will the US Federal Reserve cut interest rates in Q3 2023?",
  "Will Tesla stock outperform the S&P 500 in 2023?",
  "Will a Democrat win the 2024 US presidential election?",
  "Will OpenAI release GPT-5 before the end of 2023?",
  "Will the next iPhone include a foldable screen?",
  "Will SpaceX successfully launch Starship to orbit in 2023?",
  "Will META stock reach a new all-time high in 2023?",
  "Will oil prices exceed $100 per barrel in Q2 2023?",
  "Will NVIDIA stock performance exceed AMD stock performance in 2023?",
  "Will the Fed announce another 75 basis point rate hike before September 2023?",
  "Will the average global temperature in 2023 exceed the average in 2022?",
  "Will India's GDP growth exceed China's GDP growth in fiscal year 2023?",
  "Will the US unemployment rate be below 4% at the end of 2023?"
];

const MARKET_OUTCOMES = [
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No", "Unchanged"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["NVIDIA", "AMD", "Equal Performance"],
  ["Yes", "No"],
  ["Yes", "No", "Exactly Equal"],
  ["Yes", "No", "Equal"],
  ["Yes", "No"]
];

async function generateTestData() {
  console.log("Generating test data for Foresight Protocol on Devnet...");
  
  // Set up connection to Devnet
  const connection = new anchor.web3.Connection("https://api.devnet.solana.com", "confirmed");
  
  // Generate keypairs
  const adminKeypair = Keypair.generate();
  const participantKeypairs = Array(PARTICIPANTS_COUNT).fill(0).map(() => Keypair.generate());
  
  console.log("Admin public key:", adminKeypair.publicKey.toString());
  
  for (let i = 0; i < participantKeypairs.length; i++) {
    console.log(`Participant ${i+1} public key:`, participantKeypairs[i].publicKey.toString());
  }
  
  // Fund accounts with SOL - reduced amounts for devnet
  console.log("\nFunding accounts with SOL...");
  await fundAccounts([
    {keypair: adminKeypair, amount: 1 * LAMPORTS_PER_SOL}, // 1 SOL for admin
    ...participantKeypairs.map(keypair => ({keypair, amount: 0.5 * LAMPORTS_PER_SOL})) // 0.5 SOL for each participant
  ], connection);
  
  // Create provider and client
  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(adminKeypair),
    { commitment: "confirmed" }
  );
  
  const client = new PredictionMarketClient(provider);
  console.log("Connected to program with ID:", client.program.programId.toString());
  
  try {
    // Initialize Protocol Stats
    console.log("\nInitializing Protocol Stats...");
    try {
      const txInitProtocolStats = await client.initializeProtocolStats(adminKeypair);
      console.log("✅ Protocol stats initialized:", txInitProtocolStats);
    } catch (error) {
      if (error.message?.includes("already in use")) {
        console.log("🔄 Protocol stats already initialized, continuing...");
      } else {
        throw error;
      }
    }
    
    // Initialize AI resolver
    console.log("\nInitializing AI resolver...");
    try {
      const txInitAiResolver = await client.initializeAiResolver(adminKeypair);
      console.log("✅ AI resolver initialized:", txInitAiResolver);
    } catch (error) {
      if (error.message?.includes("already in use")) {
        console.log("🔄 AI resolver already initialized, continuing...");
      } else {
        throw error;
      }
    }
    
    // Create a token mint (for all markets)
    console.log("\nCreating token mint...");
    const mint = await createMint(
      connection,
      adminKeypair,
      adminKeypair.publicKey,
      adminKeypair.publicKey,
      6 // decimals
    );
    console.log("✅ Mint created:", mint.toString());
    
    // Create Profiles for all participants
    console.log("\nCreating profiles for all participants...");
    const creatorProfiles = [];
    const userProfiles = [];
    const tokenAccounts = [];
    
    for (let i = 0; i < participantKeypairs.length; i++) {
      const participant = participantKeypairs[i];
      
      // Create creator profile
      try {
        const txCreateCreatorProfile = await client.createCreatorProfile(participant);
        creatorProfiles.push(await client.findCreatorProfileAddress(participant.publicKey));
        console.log(`✅ Creator profile created for participant ${i+1}:`, txCreateCreatorProfile);
      } catch (error) {
        if (error.message?.includes("already in use")) {
          console.log(`🔄 Creator profile for participant ${i+1} already exists, continuing...`);
          creatorProfiles.push(await client.findCreatorProfileAddress(participant.publicKey));
        } else {
          throw error;
        }
      }
      
      // Create user profile
      try {
        const txCreateUserProfile = await client.initializeUserProfile(participant);
        userProfiles.push(await client.findUserProfileAddress(participant.publicKey));
        console.log(`✅ User profile created for participant ${i+1}:`, txCreateUserProfile);
      } catch (error) {
        if (error.message?.includes("already in use")) {
          console.log(`🔄 User profile for participant ${i+1} already exists, continuing...`);
          userProfiles.push(await client.findUserProfileAddress(participant.publicKey));
        } else {
          throw error;
        }
      }
      
      // Create token account for participant
      const participantAta = await getOrCreateAssociatedTokenAccount(
        connection,
        participant,
        mint,
        participant.publicKey
      );
      
      tokenAccounts.push(participantAta);
      
      // Mint tokens to participant - reduced amount
      await mintTo(
        connection,
        adminKeypair,
        mint,
        participantAta.address,
        adminKeypair.publicKey,
        100_000_000 // 100 tokens
      );
    }
    
    console.log("✅ All profiles created and token accounts funded");
    
    // Create Markets
    console.log("\nCreating markets...");
    const marketsInfo = [];
    let marketIndex = 0;
    
    for (let participantIdx = 0; participantIdx < participantKeypairs.length; participantIdx++) {
      const participant = participantKeypairs[participantIdx];
      
      // Each participant creates markets (mix of timebound and openended)
      for (let i = 0; i < MARKETS_PER_PARTICIPANT; i++) {
        if (marketIndex >= MARKET_QUESTIONS.length) break;
        
        const question = MARKET_QUESTIONS[marketIndex];
        const outcomes = MARKET_OUTCOMES[marketIndex];
        const aiScore = 0.85 * 100; // Convert to 0-100 scale
        const resolutionTime = new anchor.BN(Math.floor(Date.now() / 1000) + 30 * 24 * 60 * 60); // 30 days from now
        const marketType = i < 2 ? 0 : 1; // First two are timebound (0), last one is openended (1)
        const creatorMetadata = "Created via test data generator";
        
        try {
          const txCreateMarket = await client.createMarket(
            participant,
            mint,
            question,
            outcomes,
            aiScore,
            resolutionTime,
            marketType,
            creatorMetadata,
            undefined, // fee is determined by tier
            true // AI resolvable
          );
          
          // Find the market address
          const [creatorProfileAddress] = await client.findCreatorProfileAddress(participant.publicKey);
          const creatorProfile = await client.program.account.creatorProfile.fetch(creatorProfileAddress);
          
          // Need to adjust for the just-created market
          const thisMarketIndex = creatorProfile.marketsCreated - 1;
          const [marketAddress] = await client.findMarketAddress(participant.publicKey, thisMarketIndex);
          
          marketsInfo.push({
            publicKey: marketAddress.toString(),
            creator: participant.publicKey.toString(),
            question: question,
            outcomes: outcomes,
            marketType: marketType,
            index: marketIndex
          });
          
          console.log(`✅ Market ${marketIndex+1} created: ${marketAddress.toString()}`);
        } catch (error) {
          console.error(`Error creating market "${question}":`, error);
        }
        
        marketIndex++;
      }
    }
    
    // Create predictions
    console.log("\nCreating predictions...");
    const predictionsInfo = [];
    
    for (let participantIdx = 0; participantIdx < participantKeypairs.length; participantIdx++) {
      const participant = participantKeypairs[participantIdx];
      const participantAta = tokenAccounts[participantIdx];
      
      // Each participant makes predictions on 3 markets, but not their own
      for (let i = 0; i < 3; i++) {
        // Calculate which market to predict on - avoid own markets
        const marketToPredict = (participantIdx * MARKETS_PER_PARTICIPANT + i + MARKETS_PER_PARTICIPANT) % marketsInfo.length;
        const market = marketsInfo[marketToPredict];
        const marketPubkey = new PublicKey(market.publicKey);
        const outcomeIndex = i % market.outcomes.length; // Distribute predictions among outcomes
        const amount = new anchor.BN(0.2 * 1_000_000); // 0.2 tokens (adjusted for 6 decimals)
        
        try {
          const txStakePrediction = await client.stakePrediction(
            participant,
            marketPubkey,
            participantAta.address,
            outcomeIndex,
            amount
          );
          
          // Get the prediction PDA address
          const [predictionAddress] = await client.findPredictionAddress(marketPubkey, participant.publicKey);
          
          predictionsInfo.push({
            publicKey: predictionAddress.toString(),
            user: participant.publicKey.toString(),
            market: marketPubkey.toString(),
            outcomeIndex,
            amount: amount.toString()
          });
          
          console.log(`✅ Prediction created for participant ${participantIdx+1} on market ${marketToPredict+1}: ${predictionAddress.toString()}`);
        } catch (error) {
          console.error(`Error staking prediction for participant ${participantIdx+1} on market ${marketToPredict+1}:`, error);
        }
      }
    }
    
    // Save generated data to JSON file
    const generatedData = {
      admin: {
        publicKey: adminKeypair.publicKey.toString(),
        secretKey: Array.from(adminKeypair.secretKey)
      },
      participants: participantKeypairs.map((keypair, i) => ({
        index: i,
        publicKey: keypair.publicKey.toString(),
        secretKey: Array.from(keypair.secretKey)
      })),
      mint: mint.toString(),
      markets: marketsInfo,
      predictions: predictionsInfo
    };
    
    fs.writeFileSync('generated_data.json', JSON.stringify(generatedData, null, 2));
    console.log("\nGenerated data saved to generated_data.json");
    
    // Create a wallet-compatible format for the keys
    const walletKeys = {
      admin: {
        publicKey: adminKeypair.publicKey.toString(),
        secretKey: bs58.encode(adminKeypair.secretKey)
      },
      participants: participantKeypairs.map((keypair, i) => ({
        index: i,
        publicKey: keypair.publicKey.toString(),
        secretKey: bs58.encode(keypair.secretKey)
      }))
    };
    
    fs.writeFileSync('wallet_keys.json', JSON.stringify(walletKeys, null, 2));
    console.log("Wallet-compatible keys saved to wallet_keys.json");
    
    console.log("\nTest data generation complete!");
    return generatedData;
    
  } catch (error) {
    console.error("Error generating test data:", error);
    throw error;
  }
}

// Helper function to fund accounts
async function fundAccounts(accounts: {keypair: Keypair, amount: number}[], connection: anchor.web3.Connection) {
  for (const {keypair, amount} of accounts) {
    try {
      const signature = await connection.requestAirdrop(
        keypair.publicKey,
        amount
      );
      await connection.confirmTransaction(signature, "confirmed");
      console.log(`Funded ${keypair.publicKey.toString()} with ${amount / LAMPORTS_PER_SOL} SOL`);
    } catch (error) {
      console.error(`Error funding ${keypair.publicKey.toString()}:`, error);
    }
  }
}

// Execute the main function
generateTestData().then(() => {
  console.log("Script execution completed successfully!");
}).catch(error => {
  console.error("Script execution failed:", error);
});