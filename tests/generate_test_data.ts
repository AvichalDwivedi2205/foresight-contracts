import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { PredictionMarketClient } from "./contracts";
import * as fs from 'fs';
import * as bs58 from 'bs58';


const MARKET_DISTRIBUTION = {
  timebound: 20,
  openended: 10,
  total: 30
};

const PARTICIPANTS_COUNT = 5;
const MARKETS_PER_PARTICIPANT = 6;

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
  "Will Twitter implement a new paid subscription tier in 2023?",
  "Will the Euro strengthen against the USD by the end of 2023?",
  "Will a major tech company announce significant layoffs in Q3 2023?",
  "Will commercial quantum computing achieve 'quantum advantage' for a practical application in 2023?",
  "Will China's economy grow by more than 5% in 2023?",
  "Will the EU approve new copyright legislation affecting AI training by the end of 2023?",
  "Will the price of gold exceed $2,500 per ounce in 2023?",
  "Will the next major smartphone release include a significant AI feature?",
  "Will any cryptocurrency besides Bitcoin reach a $500B market cap in 2023?",
  "Will OpenAI release a video generation model before the end of 2023?",
  "Will a viable nuclear fusion breakthrough be announced in 2023?",
  "Will a major streaming service acquire a traditional media company in 2023?",
  "Will the US implement new federal AI regulations before the end of 2023?",
  "Will electric vehicles account for more than 15% of global car sales in 2023?",
  "Will a space tourism company successfully launch a civilian crew to orbit in 2023?",
  "Will DeFi protocols collectively hold over $100B in TVL by end of 2023?"
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
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"],
  ["Yes", "No"]
];

async function generateTestData() {
  console.log("Generating test data for Foresight Protocol on Devnet...");
  
  const connection = new anchor.web3.Connection("https://api.devnet.solana.com", "confirmed");
  
  console.log("Using predefined keypairs from wallet_keys.json");
  
  const adminKeypair = Keypair.fromSecretKey(
    bs58.decode("your key")
  );
  
  const participantKeypairs = [
    Keypair.fromSecretKey(bs58.decode("Your key")), 
    Keypair.fromSecretKey(bs58.decode("your key")), 
    Keypair.fromSecretKey(bs58.decode("your key")), 
    Keypair.fromSecretKey(bs58.decode("your key")), 
    Keypair.fromSecretKey(bs58.decode("your key"))  
  ];
  
  console.log("Admin public key:", adminKeypair.publicKey.toString());
  
  for (let i = 0; i < participantKeypairs.length; i++) {
    console.log(`Participant ${i+1} public key:`, participantKeypairs[i].publicKey.toString());
  }
  
  console.log("\nFunding accounts with SOL...");
  await fundAccounts([
    {keypair: adminKeypair, amount: 1 * LAMPORTS_PER_SOL},
    ...participantKeypairs.map(keypair => ({keypair, amount: 0.5 * LAMPORTS_PER_SOL}))
  ], connection);
  
  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(adminKeypair),
    { commitment: "confirmed" }
  );
  
  const client = new PredictionMarketClient(provider);
  console.log("Connected to program with ID:", client.program.programId.toString());
  
  try {
    console.log("\nInitializing Protocol Stats...");
    try {
      const txInitProtocolStats = await client.initializeProtocolStats(adminKeypair);
      console.log("Protocol stats initialized:", txInitProtocolStats);
    } catch (error) {
      if (error.message?.includes("already in use")) {
        console.log("Protocol stats already initialized, continuing...");
      } else {
        throw error;
      }
    }
    
    console.log("\nInitializing AI resolver...");
    try {
      const txInitAiResolver = await client.initializeAiResolver(adminKeypair);
      console.log("AI resolver initialized:", txInitAiResolver);
    } catch (error) {
      if (error.message?.includes("already in use")) {
        console.log("AI resolver already initialized, continuing...");
      } else {
        throw error;
      }
    }
    
    console.log("\nCreating token mint...");
    const mint = await createMint(
      connection,
      adminKeypair,
      adminKeypair.publicKey,
      adminKeypair.publicKey,
      6 // decimals
    );
    console.log("Mint created:", mint.toString());
    
    console.log("\nCreating profiles for all participants...");
    const creatorProfiles = [];
    const userProfiles = [];
    const tokenAccounts = [];
    
    for (let i = 0; i < participantKeypairs.length; i++) {
      const participant = participantKeypairs[i];
      
      try {
        const txCreateCreatorProfile = await client.createCreatorProfile(participant);
        creatorProfiles.push(await client.findCreatorProfileAddress(participant.publicKey));
        console.log(`Creator profile created for participant ${i+1}:`, txCreateCreatorProfile);
      } catch (error) {
        if (error.message?.includes("already in use")) {
          console.log(`Creator profile for participant ${i+1} already exists, continuing...`);
          creatorProfiles.push(await client.findCreatorProfileAddress(participant.publicKey));
        } else {
          throw error;
        }
      }
      
      try {
        const txCreateUserProfile = await client.initializeUserProfile(participant);
        userProfiles.push(await client.findUserProfileAddress(participant.publicKey));
        console.log(`User profile created for participant ${i+1}:`, txCreateUserProfile);
      } catch (error) {
        if (error.message?.includes("already in use")) {
          console.log(`User profile for participant ${i+1} already exists, continuing...`);
          userProfiles.push(await client.findUserProfileAddress(participant.publicKey));
        } else {
          throw error;
        }
      }
      
      const participantAta = await getOrCreateAssociatedTokenAccount(
        connection,
        participant,
        mint,
        participant.publicKey
      );
      
      tokenAccounts.push(participantAta);
      
      await mintTo(
        connection,
        adminKeypair,
        mint,
        participantAta.address,
        adminKeypair.publicKey,
        100_000_000 // 100 tokens
      );
    }
    
    console.log("All profiles created and token accounts funded");
    
    console.log("\nCreating markets...");
    const marketsInfo = [];
    let marketIndex = 0;
    
    for (let participantIdx = 0; participantIdx < participantKeypairs.length; participantIdx++) {
      const participant = participantKeypairs[participantIdx];
      
      for (let i = 0; i < MARKETS_PER_PARTICIPANT; i++) {
        if (marketIndex >= MARKET_QUESTIONS.length) break;
        
        const question = MARKET_QUESTIONS[marketIndex];
        const outcomes = MARKET_OUTCOMES[marketIndex];
        const aiScore = 0.85 * 100; // Convert to 0-100 scale
        const resolutionTime = new anchor.BN(Math.floor(Date.now() / 1000) + 30 * 24 * 60 * 60); // 30 days from now
        const marketType = i < 4 ? 0 : 1; // First four are timebound (0), last two are openended (1)
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
          
          const [creatorProfileAddress] = await client.findCreatorProfileAddress(participant.publicKey);
          const creatorProfile = await client.program.account.creatorProfile.fetch(creatorProfileAddress);
          
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
          
          console.log(`Market ${marketIndex+1} created: ${marketAddress.toString()}`);
        } catch (error) {
          console.error(`Error creating market "${question}":`, error);
        }
        
        marketIndex++;
      }
    }
    
    console.log("\nCreating predictions...");
    const predictionsInfo = [];
    
    for (let participantIdx = 0; participantIdx < participantKeypairs.length; participantIdx++) {
      const participant = participantKeypairs[participantIdx];
      const participantAta = tokenAccounts[participantIdx];
      
      for (let i = 0; i < 3; i++) {
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
          
          const [predictionAddress] = await client.findPredictionAddress(marketPubkey, participant.publicKey);
          
          predictionsInfo.push({
            publicKey: predictionAddress.toString(),
            user: participant.publicKey.toString(),
            market: marketPubkey.toString(),
            outcomeIndex,
            amount: amount.toString()
          });
          
          console.log(`Prediction created for participant ${participantIdx+1} on market ${marketToPredict+1}: ${predictionAddress.toString()}`);
        } catch (error) {
          console.error(`Error staking prediction for participant ${participantIdx+1} on market ${marketToPredict+1}:`, error);
        }
      }
    }
    
    const generatedData = {
      admin: {
        publicKey: adminKeypair.publicKey.toString(),
        secretKey: "[REDACTED]"
      },
      creators: participantKeypairs.map((keypair, i) => ({
        index: i,
        publicKey: keypair.publicKey.toString(),
        secretKey: "[REDACTED]"
      })),
      mint: mint.toString(),
      markets: marketsInfo,
      predictions: predictionsInfo
    };
    
    const secureData = {
      admin: {
        publicKey: adminKeypair.publicKey.toString(),
        secretKey: Array.from(adminKeypair.secretKey)
      },
      creators: participantKeypairs.map((keypair, i) => ({
        index: i,
        publicKey: keypair.publicKey.toString(),
        secretKey: Array.from(keypair.secretKey)
      })),
    };
    
    fs.writeFileSync('generated_data.json', JSON.stringify(generatedData, null, 2));
    console.log("\nGenerated data saved to generated_data.json with masked private keys");
    
    fs.writeFileSync('secure_keys.json', JSON.stringify(secureData, null, 2));
    console.log("Actual keys saved to secure_keys.json - KEEP THIS FILE SECURE");
    
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

async function fundAccounts(accounts: {keypair: Keypair, amount: number}[], connection: anchor.web3.Connection) {
  for (const {keypair, amount} of accounts) {
    try {
      const balance = await connection.getBalance(keypair.publicKey);
      
      if (balance >= amount) {
        console.log(`Account ${keypair.publicKey.toString()} already has ${balance / LAMPORTS_PER_SOL} SOL, skipping airdrop`);
        continue;
      }
      
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

generateTestData().then(() => {
  console.log("Script execution completed successfully!");
}).catch(error => {
  console.error("Script execution failed:", error);
});