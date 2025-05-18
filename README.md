# Foresight Protocol Smart Contracts

This directory contains the Solana smart contracts (programs) that power the Foresight Protocol prediction market platform.

## Overview

Foresight Protocol is a decentralized prediction market platform built on Solana that allows users to create markets, stake on outcomes, and earn rewards for accurate predictions. The platform features an innovative AI resolver mechanism for market resolution, creator tiers for fee structures, and community voting for open-ended markets.

## Core Components

### Account Structures

1. **Market** - Represents a prediction market:
   - `creator`: The market creator's public key
   - `question`: The prediction market question
   - `outcomes`: Array of possible outcomes (typically "Yes" and "No" for binary markets)
   - `ai_score`: AI-generated quality score (0.0 to 1.0)
   - `market_type`: Type of market (0 = Time-bound, 1 = Open-ended)
   - `deadline`: The timestamp when the market expires
   - `resolved`: Whether the market has been resolved
   - `winning_outcome`: The index of the winning outcome
   - `total_pool`: Total amount staked across all outcomes
   - `creator_fee_bps`: Creator's fee in basis points (100 = 1%)
   - `protocol_fee_bps`: Protocol fee in basis points
   - `stakes_per_outcome`: Array of staked amounts per outcome
   - `ai_resolvable`: Whether the market can be resolved by AI

2. **Prediction** - Represents a user's stake on an outcome:
   - `user`: User's public key
   - `market`: Market public key
   - `outcome_index`: The outcome index the user predicted
   - `amount`: Amount staked
   - `timestamp`: When the prediction was made
   - `claimed`: Whether rewards have been claimed

3. **CreatorProfile** - Tracks creator stats and tier:
   - `creator`: Creator's public key
   - `markets_created`: Number of markets created
   - `total_volume`: Total volume across all created markets
   - `traction_score`: Score based on market performance
   - `tier`: Creator tier (0-5) determining fees

4. **UserProfile** - Tracks user statistics:
   - `user`: User's public key
   - `total_staked`: Total amount staked
   - `total_winnings`: Total amount won
   - `total_predictions`: Number of predictions made
   - `winning_predictions`: Number of winning predictions

5. **AIResolver** - Manages AI resolution authority:
   - `authority`: Authority public key
   - `active`: Whether resolver is active
   - `resolution_count`: Number of markets resolved

### Market Types

#### Time-bound Markets
- Have a specific expiration date
- Can be resolved by AI or admin after expiration
- Resolution is based on objective, verifiable outcomes
- Example: "Will Bitcoin exceed $100,000 by December 31, 2024?"

#### Open-ended Markets
- Do not have a specific expiration date
- Resolved through community voting mechanism
- Suitable for long-term predictions with unclear timeframes
- Example: "Will humans establish a permanent colony on Mars?"

### Creator Tier System

The platform implements a progressive tier system for market creators:

| Tier | Level | Fee (BPS) | Requirements |
|------|-------|-----------|-------------|
| 0 | Beginner | 150 (1.5%) | None |
| 1 | Rising | 175 (1.75%) | 5+ markets, 1,000+ volume |
| 2 | Established | 200 (2%) | 20+ markets, 10,000+ volume |
| 3 | Expert | 300 (3%) | 50+ markets, 50,000+ volume, 500+ traction |
| 4 | Elite | 400 (4%) | 100+ markets, 200,000+ volume, 1,000+ traction |
| 5 | Master | 500 (5%) | 200+ markets, 500,000+ volume, 2,000+ traction |

Traction score increases based on market activity and successful resolutions.

## AI Resolution Mechanism

The Foresight Protocol features an innovative AI resolution system that:

1. **Validates Market Quality**: Evaluates market questions for clarity, measurability, and appropriate timeframe
2. **Scores Market Questions**: Provides a quality score from 0.0 to 1.0
3. **Resolves Time-bound Markets**: Can automatically resolve markets with high confidence (>85%)
4. **Provides Resolution Data**: Documents evidence and confidence level for transparency

AI resolution is only applied to time-bound markets that meet specific criteria for objective, verifiable outcomes.

## Community Voting System

For open-ended markets or when AI resolution is challenged:

1. **Voting Period**: Opens after market deadline for 15 days
2. **Stake-weighted Voting**: Users who staked can vote with weight proportional to their stake
3. **Resolution Proposal**: Authorities can propose outcomes after voting period
4. **Challenge Period**: 48-hour window to challenge proposed resolutions
5. **Final Resolution**: Determines outcome based on stake-weighted votes if challenged

## Security Features

1. **Admin Controls**: Restricted functions only callable by authorized admin
2. **PDA-based Accounts**: Uses Program Derived Addresses for secure, deterministic account creation
3. **Input Validation**: Thorough validation of all input parameters
4. **Safe Math Operations**: Uses checked arithmetic to prevent overflow/underflow
5. **Event Emission**: Emits events for transparent tracking of all critical operations

## Getting Started

### Prerequisites
- Solana CLI tools
- Anchor framework

### Building the Program
```bash
cd contracts
anchor build
```

### Testing
```bash
anchor test
```

### Deployment
```bash
anchor deploy
```

## Oracle Service

The Foresight Protocol includes an Oracle service responsible for:

1. **AI Market Validation**: Analyzes proposed market questions for validity and quality
2. **AI Market Resolution**: Resolves time-bound markets with verifiable outcomes
3. **Data Aggregation**: Collects and analyzes data from multiple sources
4. **Safe Execution**: Ensures secure, auditable resolution process

### Oracle Architecture

The Oracle service consists of:

- **AI Resolver Account**: On-chain account that tracks resolution authority
- **Resolution Authority**: Permissioned entity responsible for submitting resolutions
- **Resolution Process**: Multi-step verification workflow for outcome determination

### Oracle Initialization

```rust
pub fn initialize_ai_resolver(
    ctx: Context<InitializeAIResolver>,
) -> Result<()> {
    // Validate that only the authorized admin can initialize the resolver
    require!(is_admin(&ctx.accounts.admin.key()), ErrorCode::Unauthorized);
    
    let resolver = &mut ctx.accounts.ai_resolver;
    resolver.authority = ctx.accounts.admin.key();
    resolver.active = true;
    resolver.resolution_count = 0;
    resolver.bump = ctx.bumps.ai_resolver;
    
    msg!("AI resolver initialized with authority: {}", resolver.authority);
    Ok(())
}
```

### Market Resolution Flow

```rust
pub fn resolve_market_via_ai(
    ctx: Context<ResolveMarketViaAI>,
    winning_outcome_index: u8,
    ai_confidence_score: f32,
    resolution_data: String,
) -> Result<()> {
    // Validate authority, active status, market eligibility
    // ...
    
    // Require high confidence score
    require!(ai_confidence_score >= 0.85, ErrorCode::LowAIConfidence);
    
    // Set winning outcome and mark as resolved
    market.winning_outcome = Some(winning_outcome_index);
    market.resolved = true;
    
    // Increment resolution count
    ctx.accounts.ai_resolver.resolution_count = ctx.accounts.ai_resolver.resolution_count.checked_add(1).unwrap();
    
    msg!("Market resolved by AI with outcome: {}, confidence: {}", winning_outcome_index, ai_confidence_score);
    msg!("Resolution data: {}", resolution_data);
    
    Ok(())
}
```

## License

[MIT](LICENSE) 