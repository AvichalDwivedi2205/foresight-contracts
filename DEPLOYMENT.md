# Foresight Protocol Deployment Guide

This guide explains how to deploy the Foresight Protocol smart contract to Solana Devnet and set up testing data.

## Prerequisites

1. [Solana CLI tools](https://docs.solana.com/cli/install-solana-cli-tools) installed
2. [Anchor](https://www.anchor-lang.com/) installed
3. A Solana wallet with enough devnet SOL (at least 4 SOL for program deployment)

## Deployment Steps

### 1. Configure your wallet

Make sure you have a Solana wallet set up. If you don't have one, create a new keypair:

```bash
solana-keygen new --outfile ~/.config/solana/id.json
```

### 2. Fund your wallet with devnet SOL

Deploying a Solana program requires approximately 4 SOL. You can get SOL from:

- **Solana Devnet Faucet**: Visit https://faucet.solana.com/ and request SOL
- **Command line**: Run multiple airdrops (2 SOL limit per request)

```bash
solana airdrop 2 --url devnet
# Wait a moment between requests
solana airdrop 2 --url devnet
```

Check your balance:

```bash
solana balance --url devnet
```

### 3. Install dependencies

```bash
cd contracts
npm install
```

### 4. Deploy the contract to devnet

Our deployment script will:
- Check if you have enough SOL
- Attempt to airdrop more SOL if needed
- Build and deploy the contract

```bash
npm run deploy
```

Or use the Anchor command directly:

```bash
anchor deploy --provider.cluster devnet
```

### 5. Generate test data (optional)

After deploying, you can generate test data:

```bash
npm run generate-data
```

This will:
- Create 5 participants with both creator and user profiles
- Fund each participant with 0.5 SOL
- Each participant will create 3 prediction markets
- Participants will make predictions on each other's markets

The script will output wallet keys and test data to:
- `generated_data.json`: Full test data
- `wallet_keys.json`: Wallet-compatible keys

### 6. Use the contract on your frontend

Update your frontend environment with the appropriate configuration:

```
NEXT_PUBLIC_PROGRAM_ID=7Gh4eFGmobz5ngu2U3bgZiQm2Adwm33dQTsUwzRb7wBi
NEXT_PUBLIC_CLUSTER=devnet
```

## Troubleshooting

### Insufficient funds error

If you see an error like:

```
Error: Account has insufficient funds for spend (X SOL) + fee (Y SOL)
```

You need to add more SOL to your wallet. Try:

1. Using the Solana Faucet: https://faucet.solana.com/
2. Multiple airdrop commands with pauses between them
3. If that doesn't work, you might need to request SOL from a team member or use a different wallet

### Rate limiting errors

Devnet has rate limits for transactions and airdrops. If you encounter rate limiting:

1. Wait 10-15 minutes before trying again
2. Split your operations into smaller batches
3. Add pauses between transactions in your scripts

## Tips for Devnet

1. Be mindful of devnet rate limits. The network might reject transactions if you send too many at once.
2. Devnet airdrops are limited to 2 SOL per request.
3. Use smaller token amounts (we've adjusted the test script to use 100 tokens instead of 1000).
4. If you encounter issues, check your Solana logs with: `solana logs --url devnet <PROGRAM_ID>`

## Integrating the Frontend

To integrate your frontend app with the deployed contract:

1. If your frontend is in a separate repository, make sure to reference the correct program ID in your connection settings.
2. Alternatively, you can move your frontend code into the `app` directory within the contract folder for better organization.

## Viewing Your Contract

You can view your deployed program on the Solana Explorer:
https://explorer.solana.com/address/7Gh4eFGmobz5ngu2U3bgZiQm2Adwm33dQTsUwzRb7wBi?cluster=devnet 