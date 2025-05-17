/**
 * IMPORTANT: This file contains placeholder keys that will not work for actual transactions.
 * You must replace them with your own keys before using this script.
 * Please refer to KEYS_README.md for instructions on setting up your keys.
 */

import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { PredictionMarketClient } from "./contracts";
import * as fs from 'fs';
import * as bs58 from 'bs58';
import * as path from 'path';

const MARKET_DISTRIBUTION = {
  timebound: 80,
  openended: 20,
  total: 100
};

const MARKETS_PER_CREATOR = [21, 6, 2, 1, 0];

const MARKET_QUESTIONS = [
  "Will a DAO try to buy a fast-food chain by the end of 2025?",
  "Will an AI influencer win a 'real' human dating show by Q4 2025?",
  "Will someone accidentally send $10M in crypto to a smart contract with no withdrawal function in 2025?",
  "Will a Solana meme coin be accepted at a real funeral home in 2025?",
  "Will a crypto startup pivot to 'AI for pets' and raise $50M in 2025?",
  "Will a metaverse couple get legally married in multiple jurisdictions in 2025?",
  "Will someone propose using NFTs to track sandwich ownership in 2025?",
  "Will a Web3 project win a Grammy before the SEC finishes investigating it?",
  "Will a 'ChatGPT but on Solana' project raise a Series A with no working prototype by end of 2025?",
  "Will an AI-generated DAO sue itself in 2025?",
  "Will Vitalik Buterin wear a meme shirt that moves SOL price by over 5% in 2025?",
  "Will someone create a Solana gas fee nostalgia simulator by 2025?",
  "Will Elon Musk tweet 'Solana' and cause a network outage in 2025?",
  "Will a DAO launch to fight another DAO that fights DAOs in 2025?",
  "Will a Solana-powered vending machine accept doge memes as valid payment?",
  "Will any project pitch 'Uber for DAOs' at a 2025 hackathon?",
  "Will someone lose a private key tattooed in a QR code due to sunburn by end of 2025?",
  "Will an NFT of a rock sell for more than a real house in 2025?",
  "Will a Solana dApp require solving a riddle to log in by end of 2025?",
  "Will a Discord server become legally recognized as a governing body in 2025?",
  "Will a crypto project add 'AI', 'quantum', and 'climate' to its name in a single rebrand in 2025?",
  "Will a Solana meme coin be used to pay rent anywhere on Earth in 2025?",
  "Will a hackathon team ship an app whose only feature is 'connect wallet' in 2025?",
  "Will an NFT receipt be used as evidence in small claims court in 2025?",
  "Will someone build a decentralized app to verify whether a meme is dank?",
  "Will a Solana-based dating app match people by wallet compatibility?",
  "Will a dev accidentally deploy a production contract with `console.log()` left in by end of 2025?",
  "Will a DAO vote on whether pineapple belongs on pizza by December 2025?",
  "Will a crypto bro use 'on-chain vibes' as legal defense in court by end of 2025?",
  "Will someone fork a Solana DeFi protocol to run a lemonade stand in 2025?"
];

const MARKET_OUTCOMES = [
  ["Yes", "No", "Only a food truck"],
  ["Yes", "No", "Catfished everyone"],
  ["Yes", "No", "Already happened"],
  ["Yes", "No", "Memorial DAO instead"],
  ["Yes", "No", "IPO on DogeChain"],
  ["Yes", "No", "Annulled on-chain"],
  ["Yes", "No", "Already VC-funded"],
  ["Yes", "No", "Both simultaneously"],
  ["Yes", "No", "Acquired by OpenAI"],
  ["Yes", "No", "Settled out of court (on-chain)"],
  ["Yes", "No", "Shirt was AI-generated"],
  ["Yes", "No", "L2 flashbacks instead"],
  ["Yes", "No", "Tweet was deleted"],
  ["Yes", "No", "Merged into a SuperDAO"],
  ["Yes", "No", "NFT receipt only"],
  ["Yes", "No", "Already trademarked"],
  ["Yes", "No", "Tattoo faded away"],
  ["Yes", "No", "Technically a garage"],
  ["Yes", "No", "Only if wallet is funded"],
  ["Yes", "No", "Unofficially recognized"],
  ["Yes", "No", "Added 'Metaverse' too"],
  ["Yes", "No", "Only to landlord's alt wallet"],
  ["Yes", "No", "Still won a bounty"],
  ["Yes", "No", "Judge was confused"],
  ["Yes", "No", "Algorithm said 'mid'"],
  ["Yes", "No", "No mutual staking detected"],
  ["Yes", "No", "Gas fees too high to fix"],
  ["Yes", "No", "Pineapple DAO forked"],
  ["Yes", "No", "Judge laughed it off"],
  ["Yes", "No", "Profitable in lemonade tokens"]
];

const MARKET_POPULARITY = [
  8, 7, 10, 9, 3, 6, 5, 7, 8, 10,
  4, 6, 9, 7, 8, 10, 9, 7, 8, 6,
  5, 3, 7, 9, 4, 2, 8, 6, 5, 3
];

const ADDITIONAL_MARKET_QUESTIONS = [
  "Will AI hallucinations be legally recognized as a form of art by end of 2025?",
  "Will a smart contract win a Pulitzer Prize for journalism in 2025?",
  "Will Solana governance vote to include a 'while you were away' notification feature in 2025?",
  "Will any wallet have an 'undo transaction' feature implemented by end of 2025?",
  "Will NFTs be accepted as valid ID at any government office by end of 2025?",
  "Will 'transaction failed successfully' memes see a resurgence in 2025?",
  "Will any smart contract be granted legal personhood in 2025?",
  "Will a DAO-governed restaurant have a Michelin star by end of 2025?",
  "Will whale wallets use blockchain-based psychic predictions for trading in 2025?",
  "Will someone accidentally burn a house deed NFT in 2025?"
];

const ADDITIONAL_MARKET_OUTCOMES = [
  ["Yes", "No", "Only in the metaverse"],
  ["Yes", "No", "Only an honorable mention"],
  ["Yes", "No", "Feature was too expensive"],
  ["Yes", "No", "Only for VIP addresses"],
  ["Yes", "No", "Only in crypto-friendly countries"],
  ["Yes", "No", "Replaced by 'gas optimization' memes"],
  ["Yes", "No", "Case still pending"],
  ["Yes", "No", "Food critic was paid in governance tokens"],
  ["Yes", "No", "Only for meme coins"],
  ["Yes", "No", "House value increased after burn"]
];

const ALL_MARKET_QUESTIONS = [...MARKET_QUESTIONS, ...ADDITIONAL_MARKET_QUESTIONS];
const ALL_MARKET_OUTCOMES = [...MARKET_OUTCOMES, ...ADDITIONAL_MARKET_OUTCOMES];

const ADDITIONAL_POPULARITY = Array(ADDITIONAL_MARKET_QUESTIONS.length).fill(0).map(() => Math.floor(Math.random() * 10) + 1);
const ALL_MARKET_POPULARITY = [...MARKET_POPULARITY, ...ADDITIONAL_POPULARITY];

async function withRetry<T>(
  operation: () => Promise<T>,
  maxRetries = 3,
  initialDelay = 1000
): Promise<T> {
  let retries = 0;
  let delay = initialDelay;
  
  while (true) {
    try {
      return await operation();
    } catch (error) {
      retries++;
      if (retries > maxRetries) {
        throw error;
      }
      
      console.warn(`Operation failed, retrying (${retries}/${maxRetries}) after ${delay}ms: ${error.message}`);
      await new Promise(resolve => setTimeout(resolve, delay));
      delay *= 2;
    }
  }
}

function loadKeypairsFromFile(filepath: string): { adminKeypair: Keypair | null, creatorKeypairs: Keypair[] | null } {
  try {
    if (fs.existsSync(filepath)) {
      console.log(`Loading keypairs from ${filepath}...`);
      const keysData = JSON.parse(fs.readFileSync(filepath, 'utf8'));
      
      const adminKeypair = keysData.admin ? 
        Keypair.fromSecretKey(new Uint8Array(keysData.admin)) : null;
      
      const creatorKeypairs = keysData.creators ? 
        keysData.creators.map((keyArray: number[]) => Keypair.fromSecretKey(new Uint8Array(keyArray))) : null;
      
      return { adminKeypair, creatorKeypairs };
    }
  } catch (error) {
    console.log(`Could not load keypairs from ${filepath}: ${error.message}`);
    console.log(`Please refer to KEYS_README.md for instructions on setting up your keys.`);
  }
  
  return { adminKeypair: null, creatorKeypairs: null };
}

async function generateDevnetData() {
  console.log("Generating test data for Foresight Protocol on Devnet...");
  
  const connection = new anchor.web3.Connection(
    "https://api.devnet.solana.com",
    {
      commitment: "confirmed",
      confirmTransactionInitialTimeout: 60000,
      disableRetryOnRateLimit: false,
    }
  );
  
  const { adminKeypair: loadedAdmin, creatorKeypairs: loadedCreators } = 
    loadKeypairsFromFile(path.resolve(__dirname, '../secure_keys.json'));
  
  // Replace the array below with your own admin secret key
  // This is just a placeholder and will not work for actual transactions
  const adminKeypair = loadedAdmin || Keypair.fromSecretKey(
    new Uint8Array([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ])
  );
  
  // Check if using placeholder keys and warn the user
  if (!loadedAdmin && adminKeypair.secretKey.every(byte => byte === 0)) {
    console.error("\n❌ ERROR: You're using the placeholder admin key which will not work!");
    console.error("Please set up your own keys following the instructions in KEYS_README.md");
    console.error("Exiting to prevent wasted time and resources.\n");
    process.exit(1);
  }
  
  // Replace these arrays with your own creator secret keys
  // These are just placeholders and will not work for actual transactions
  const creatorKeypairs = loadedCreators || [
    Keypair.fromSecretKey(new Uint8Array([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ])),
    Keypair.fromSecretKey(new Uint8Array([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ])),
    Keypair.fromSecretKey(new Uint8Array([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ])),
    Keypair.fromSecretKey(new Uint8Array([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ])),
    Keypair.fromSecretKey(new Uint8Array([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    ]))
  ];
  
  // Check if using placeholder keys and warn the user
  if (!loadedCreators && creatorKeypairs.some(keypair => 
      keypair.secretKey.every(byte => byte === 0))) {
    console.error("\n❌ ERROR: You're using placeholder creator keys which will not work!");
    console.error("Please set up your own keys following the instructions in KEYS_README.md");
    console.error("Exiting to prevent wasted time and resources.\n");
    process.exit(1);
  }
  
  console.log("Admin public key:", adminKeypair.publicKey.toString());
  
  for (let i = 0; i < creatorKeypairs.length; i++) {
    console.log(`Creator ${i+1} public key:`, creatorKeypairs[i].publicKey.toString());
  }
  
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
      const txInitProtocolStats = await withRetry(() => 
        client.initializeProtocolStats(adminKeypair)
      );
      console.log("✅ Protocol stats initialized:", txInitProtocolStats);
    } catch (error) {
      if (error.message?.includes("already in use")) {
        console.log("🔄 Protocol stats already initialized, continuing...");
      } else {
        throw error;
      }
    }
    
    console.log("\nInitializing AI resolver...");
    try {
      const txInitAiResolver = await withRetry(() => 
        client.initializeAiResolver(adminKeypair)
      );
      console.log("✅ AI resolver initialized:", txInitAiResolver);
    } catch (error) {
      if (error.message?.includes("already in use")) {
        console.log("🔄 AI resolver already initialized, continuing...");
      } else {
        throw error;
      }
    }
    
    let existingMint;
    try {
      const existingData = JSON.parse(fs.readFileSync('generated_data.json', 'utf8'));
      existingMint = existingData.mint;
      if (existingMint) {
        console.log("Found existing mint:", existingMint);
      }
    } catch (error) {
      console.log("No existing data found, will create a new mint");
    }
    
    console.log("\nCreating token mint...");
    let mint;
    
    if (existingMint) {
      mint = new PublicKey(existingMint);
      console.log("Using existing mint:", mint.toString());
      
      try {
        const mintInfo = await connection.getAccountInfo(mint);
        if (!mintInfo) {
          console.log("⚠️ Warning: Existing mint not found on-chain. Will create a new one.");
          existingMint = null;
        }
      } catch (error) {
        console.log("⚠️ Error checking existing mint:", error.message);
        existingMint = null;
      }
    }
    
    if (!existingMint) {
      try {
        mint = await withRetry(
          () => createMint(
            connection,
            adminKeypair,
            adminKeypair.publicKey,
            adminKeypair.publicKey,
            6
          ),
          5,
          2000
        );
        console.log("✅ New mint created:", mint.toString());
      } catch (error) {
        console.error("❌ Failed to create mint:", error);
        throw new Error("Failed to create token mint. Cannot continue without a valid mint.");
      }
    }
    
    console.log("✅ Using mint:", mint.toString());
    
    console.log("\nCreating profiles for all creators...");
    const creatorProfiles = [];
    const userProfiles = [];
    const tokenAccounts = [];
    
    for (let i = 0; i < creatorKeypairs.length; i++) {
      const creator = creatorKeypairs[i];
      
      try {
        const balance = await connection.getBalance(creator.publicKey);
        if (balance < 0.05 * LAMPORTS_PER_SOL) {
          const signature = await connection.requestAirdrop(
            creator.publicKey,
            0.2 * LAMPORTS_PER_SOL
          );
          await connection.confirmTransaction(signature);
          console.log(`🔄 Funded creator ${i+1} with 0.2 SOL`);
        }
      } catch (error) {
        console.warn(`⚠️ Could not check/fund creator ${i+1}: ${error.message}`);
      }
      
      try {
        const txCreateCreatorProfile = await withRetry(() => 
          client.createCreatorProfile(creator)
        );
        creatorProfiles.push(await client.findCreatorProfileAddress(creator.publicKey));
        console.log(`✅ Creator profile created for creator ${i+1}:`, txCreateCreatorProfile);
      } catch (error) {
        if (error.message?.includes("already in use")) {
          console.log(`🔄 Creator profile for creator ${i+1} already exists, continuing...`);
          creatorProfiles.push(await client.findCreatorProfileAddress(creator.publicKey));
        } else {
          throw error;
        }
      }
      
      try {
        const txCreateUserProfile = await withRetry(() => 
          client.initializeUserProfile(creator)
        );
        userProfiles.push(await client.findUserProfileAddress(creator.publicKey));
        console.log(`✅ User profile created for creator ${i+1}:`, txCreateUserProfile);
      } catch (error) {
        if (error.message?.includes("already in use")) {
          console.log(`🔄 User profile for creator ${i+1} already exists, continuing...`);
          userProfiles.push(await client.findUserProfileAddress(creator.publicKey));
        } else {
          throw error;
        }
      }
      
      try {
        const creatorAta = await withRetry(() => 
          getOrCreateAssociatedTokenAccount(
            connection,
            creator,
            mint,
            creator.publicKey
          )
        );
        
        tokenAccounts.push(creatorAta);
        
        try {
          const mintAmount = 100_000_000 + (i * 20_000_000);
          await withRetry(() => 
            mintTo(
              connection,
              adminKeypair,
              mint,
              creatorAta.address,
              adminKeypair.publicKey,
              mintAmount
            )
          );
          console.log(`✅ Minted ${mintAmount / 1_000_000} tokens to creator ${i+1}`);
        } catch (error) {
          console.warn(`⚠️ Could not mint tokens to creator ${i+1}: ${error.message}`);
        }
      } catch (error) {
        console.error(`❌ Failed to create/get token account for creator ${i+1}: ${error.message}`);
        tokenAccounts.push(null);
      }
    }
    
    console.log("✅ All profiles created and token accounts funded");
    
    console.log("\nCreating markets...");
    const marketsInfo = [];
    let marketIndex = 0;
    
    for (let creatorIdx = 0; creatorIdx < creatorKeypairs.length; creatorIdx++) {
      const creator = creatorKeypairs[creatorIdx];
      
      for (let i = 0; i < MARKETS_PER_CREATOR[creatorIdx]; i++) {
        if (marketIndex >= ALL_MARKET_QUESTIONS.length) break;
        
        const question = ALL_MARKET_QUESTIONS[marketIndex];
        const outcomes = ALL_MARKET_OUTCOMES[marketIndex];
        const aiScore = 0.85 * 100;
        
        const currentTime = Math.floor(Date.now() / 1000);
        const startOfYear = new Date('2025-01-01').getTime() / 1000;
        const endOfYear = new Date('2025-12-31').getTime() / 1000;
        const timeSpan = endOfYear - startOfYear;
        const resolutionTime = new anchor.BN(startOfYear + (marketIndex / ALL_MARKET_QUESTIONS.length) * timeSpan);
        
        const marketType = (marketIndex % 5 < 4) ? 0 : 1;
        const creatorMetadata = `Created by Creator ${creatorIdx + 1}`;
        
        try {
          const txCreateMarket = await withRetry(() => 
            client.createMarket(
              creator,
              mint,
              question,
              outcomes,
              aiScore,
              resolutionTime,
              marketType,
              creatorMetadata,
              undefined,
              true
            )
          );
          
          const [creatorProfileAddress] = await client.findCreatorProfileAddress(creator.publicKey);
          const creatorProfile = await withRetry(() => 
            client.program.account.creatorProfile.fetch(creatorProfileAddress)
          );
          
          const thisMarketIndex = creatorProfile.marketsCreated - 1;
          const [marketAddress] = await client.findMarketAddress(creator.publicKey, thisMarketIndex);
          
          marketsInfo.push({
            publicKey: marketAddress.toString(),
            creator: creator.publicKey.toString(),
            question: question,
            outcomes: outcomes,
            marketType: marketType,
            index: marketIndex,
            popularity: ALL_MARKET_POPULARITY[marketIndex]
          });
          
          console.log(`✅ Market ${marketIndex+1} created: ${marketAddress.toString()}`);
        } catch (error) {
          console.error(`Error creating market "${question}":`, error);
        }
        
        marketIndex++;
      }
    }
    
    console.log("\nCreating predictions...");
    const predictionsInfo = [];
    
    for (let creatorIdx = 0; creatorIdx < creatorKeypairs.length; creatorIdx++) {
      const creator = creatorKeypairs[creatorIdx];
      const creatorAta = tokenAccounts[creatorIdx];
      
      if (!creatorAta) {
        console.log(`⚠️ Skipping predictions for creator ${creatorIdx+1} due to missing token account`);
        continue;
      }
      
      for (let marketInfo of marketsInfo) {
        if (marketInfo.creator === creator.publicKey.toString()) {
          continue;
        }
        
        const popularity = marketInfo.popularity;
        const shouldPredict = Math.random() < (popularity / 10);
        
        if (shouldPredict) {
          const marketPubkey = new PublicKey(marketInfo.publicKey);
          const outcomeIndex = Math.random() < 0.6 ? 0 : (Math.random() < 0.8 ? 1 : 2);
          
          const baseAmount = 500_000 + Math.floor(Math.random() * 1_500_000);
          const popularityBoost = Math.floor((popularity / 10) * 1_000_000);
          const amount = new anchor.BN(baseAmount + popularityBoost);
          
          try {
            const txStakePrediction = await withRetry(() => 
              client.stakePrediction(
                creator,
                marketPubkey,
                creatorAta.address,
                outcomeIndex,
                amount
              )
            );
            
            const [predictionAddress] = await client.findPredictionAddress(marketPubkey, creator.publicKey);
            
            predictionsInfo.push({
              publicKey: predictionAddress.toString(),
              user: creator.publicKey.toString(),
              market: marketPubkey.toString(),
              outcomeIndex,
              amount: amount.toString()
            });
            
            console.log(`✅ Prediction created for creator ${creatorIdx+1} on market "${marketInfo.question.substring(0, 30)}...": ${predictionAddress.toString()}`);
          } catch (error) {
            console.error(`Error staking prediction for creator ${creatorIdx+1} on market ${marketInfo.publicKey}:`, error.message);
          }
        }
      }
    }
    
    const generatedData = {
      admin: {
        publicKey: adminKeypair.publicKey.toString(),
        secretKey: Array.from(adminKeypair.secretKey)
      },
      creators: creatorKeypairs.map((keypair, i) => ({
        index: i,
        publicKey: keypair.publicKey.toString(),
        secretKey: Array.from(keypair.secretKey)
      })),
      mint: mint.toString(),
      markets: marketsInfo,
      predictions: predictionsInfo
    };
    
    fs.writeFileSync('generated_data.json', JSON.stringify(generatedData, null, 2));
    console.log("\nGenerated data saved to generated_data.json with actual keys");
    
    if (!fs.existsSync('secure_keys.json')) {
      const secureKeys = {
        admin: Array.from(adminKeypair.secretKey),
        creators: creatorKeypairs.map(keypair => Array.from(keypair.secretKey))
      };
      
      fs.writeFileSync('secure_keys.json', JSON.stringify(secureKeys, null, 2));
      console.log("\nSecure keys saved to secure_keys.json for future use");
    }
    
    console.log("\nDevnet test data generation complete!");
    return generatedData;
    
  } catch (error) {
    console.error("Error generating test data:", error);
    throw error;
  }
}

generateDevnetData().then(() => {
  console.log("Script execution completed successfully!");
}).catch(error => {
  console.error("Script execution failed:", error);
});