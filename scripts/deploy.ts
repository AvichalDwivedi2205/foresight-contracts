import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Connection, Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { execSync } from "child_process";
import fs from "fs";
import path from "path";

// This script builds and deploys the contract to the Solana devnet
async function deploy() {
  console.log("Preparing to deploy contract to Solana devnet...");

  try {
    // 1. Set up connection and wallet
    const connection = new anchor.web3.Connection("https://api.devnet.solana.com", "confirmed");
    const wallet = anchor.Wallet.local();
    console.log(`Using wallet: ${wallet.publicKey.toString()}`);

    // 2. Check wallet balance
    const balance = await connection.getBalance(wallet.publicKey);
    console.log(`Current wallet balance: ${balance / LAMPORTS_PER_SOL} SOL`);

    // 3. Estimate required SOL for deployment (approximately 4 SOL)
    const requiredSol = 4 * LAMPORTS_PER_SOL;

    if (balance < requiredSol) {
      console.log(`Insufficient funds. Need approximately 4 SOL for deployment.`);
      console.log(`Current balance: ${balance / LAMPORTS_PER_SOL} SOL`);
      console.log(`Attempting to airdrop SOL (this may be rate-limited on devnet)...`);

      // Try to airdrop in chunks (devnet limits to 2 SOL per request)
      const solNeeded = requiredSol - balance;
      const airdrops = Math.ceil(solNeeded / (2 * LAMPORTS_PER_SOL));
      
      for (let i = 0; i < airdrops; i++) {
        try {
          console.log(`Airdrop attempt ${i+1}/${airdrops}...`);
          const airdropAmount = Math.min(2 * LAMPORTS_PER_SOL, solNeeded - (i * 2 * LAMPORTS_PER_SOL));
          const signature = await connection.requestAirdrop(wallet.publicKey, airdropAmount);
          await connection.confirmTransaction(signature);
          console.log(`Airdropped ${airdropAmount / LAMPORTS_PER_SOL} SOL`);
          
          // Wait a bit between airdrops to avoid rate limiting
          await new Promise(resolve => setTimeout(resolve, 2000));
        } catch (error) {
          console.error(`Error during airdrop:`, error);
          console.log(`Please fund your wallet manually with at least 4 SOL and try again.`);
          console.log(`You can request SOL from the Solana Devnet faucet: https://faucet.solana.com/`);
          return;
        }
      }
      
      // Check balance again
      const newBalance = await connection.getBalance(wallet.publicKey);
      console.log(`New wallet balance: ${newBalance / LAMPORTS_PER_SOL} SOL`);
      
      if (newBalance < requiredSol) {
        console.log(`Still insufficient funds. Please add more SOL manually and try again.`);
        return;
      }
    }

    // 4. Build the contract
    console.log("\nBuilding contract...");
    execSync("anchor build", { stdio: "inherit" });
    console.log("✅ Contract built successfully");

    // 5. Deploy to devnet
    console.log("\nDeploying to devnet...");
    execSync("anchor deploy --provider.cluster devnet", { stdio: "inherit" });
    console.log("✅ Contract deployed to devnet successfully");

    // 6. Read the program ID from Anchor.toml
    const anchorTomlPath = path.join(__dirname, "..", "Anchor.toml");
    const anchorToml = fs.readFileSync(anchorTomlPath, "utf8");
    
    // Extract program ID
    const programIdMatch = anchorToml.match(/contracts = "([^"]+)"/);
    const programId = programIdMatch ? programIdMatch[1] : null;
    
    if (!programId) {
      throw new Error("Could not find program ID in Anchor.toml");
    }
    
    console.log(`\nProgram ID: ${programId}`);
    console.log("You can view your program on Solana Explorer at:");
    console.log(`https://explorer.solana.com/address/${programId}?cluster=devnet`);
    
    // 7. Suggest generating test data
    console.log("\nNext steps:");
    console.log("1. Generate test data with: anchor run generate-data");
    console.log("2. Update your frontend to connect to the devnet contract");

  } catch (error) {
    console.error("Error during deployment:", error);
    process.exit(1);
  }
}

deploy().then(() => {
  console.log("Deployment script completed");
  process.exit(0);
}).catch(err => {
  console.error("Deployment failed:", err);
  process.exit(1);
}); 