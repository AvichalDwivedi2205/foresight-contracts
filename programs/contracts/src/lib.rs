use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use std::convert::TryFrom;

declare_id!("7Gh4eFGmobz5ngu2U3bgZiQm2Adwm33dQTsUwzRb7wBi");

/// Error codes for the Foresight Protocol
#[error_code]
pub enum ForesightError {
    #[msg("Creator cooldown period has not elapsed")]
    CreatorCooldownNotElapsed,
    #[msg("AI score is below minimum required threshold")]
    AiScoreTooLow,
    #[msg("Market deadline has not passed yet")]
    MarketDeadlineNotPassed,
    #[msg("Market has already been resolved")]
    MarketAlreadyResolved,
    #[msg("Market has not been resolved yet")]
    MarketNotResolved,
    #[msg("Only the market creator can resolve this market")]
    NotMarketCreator,
    #[msg("Only the protocol authority can perform this action")]
    NotProtocolAuthority,
    #[msg("User has already staked in this market")]
    AlreadyStaked,
    #[msg("Market deadline has already passed")]
    MarketDeadlinePassed,
    #[msg("Invalid outcome index")]
    InvalidOutcomeIndex,
    #[msg("User did not predict the winning outcome")]
    NotWinner,
    #[msg("Reward has already been claimed")]
    RewardAlreadyClaimed,
    #[msg("Calculation error")]
    CalculationError,
    #[msg("Question too long, maximum 200 characters")]
    QuestionTooLong,
    #[msg("Too many outcomes, maximum 5 allowed")]
    TooManyOutcomes,
    #[msg("Outcome text too long, maximum 50 characters each")]
    OutcomeTextTooLong,
}

#[program]
pub mod foresight_protocol {
    use super::*;

    /// Initialize a new prediction market
    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        question: String,
        outcomes: Vec<String>,
        deadline: i64,
        creator_fee_bps: u16,
        ai_score: f32,
    ) -> Result<()> {
        // Validate inputs
        require!(question.len() <= 200, ForesightError::QuestionTooLong);
        require!(outcomes.len() <= 3, ForesightError::TooManyOutcomes);
        
        for outcome in &outcomes {
            require!(outcome.len() <= 50, ForesightError::OutcomeTextTooLong);
        }

        require!(
            creator_fee_bps <= 500, // Max 5% creator fee
            ForesightError::CalculationError
        );

        let market = &mut ctx.accounts.market;
        let creator_profile = &mut ctx.accounts.creator_profile;
        let protocol_config = &ctx.accounts.protocol_config;
        let clock = &ctx.accounts.clock;

        // Check creator cooldown based on tier
        let cooldown_seconds = match creator_profile.tier {
            0 => 5 * 24 * 60 * 60, // 5 days for Tier 0
            1 => 3 * 24 * 60 * 60, // 3 days for Tier 1
            2 => 1 * 24 * 60 * 60, // 1 day for Tier 2
            _ => 0,                // No cooldown for Tier 3+
        };

        if creator_profile.last_created_at > 0 {
            let time_since_last = clock.unix_timestamp - creator_profile.last_created_at;
            require!(
                time_since_last >= cooldown_seconds,
                ForesightError::CreatorCooldownNotElapsed
            );
        }

        // Check AI validation score meets minimum threshold
        require!(
            ai_score >= protocol_config.min_ai_score,
            ForesightError::AiScoreTooLow
        );

        // Initialize market data
        market.creator = ctx.accounts.creator.key();
        market.question = question;
        market.outcomes = outcomes;
        market.deadline = deadline;
        market.created_at = clock.unix_timestamp;  // Initialize created_at field
        market.resolved = false;
        market.winning_outcome = None;
        market.total_pool = 0;
        market.total_predictions = 0;
        market.ai_score = ai_score;
        market.creator_fee_bps = creator_fee_bps;
        market.protocol_fee_bps = protocol_config.protocol_fee_bps;
        market.bump = ctx.bumps.market;

        // Update creator profile
        creator_profile.last_created_at = clock.unix_timestamp;
        creator_profile.markets_created = creator_profile.markets_created.checked_add(1)
            .ok_or(ForesightError::CalculationError)?;

        Ok(())
    }

    /// Stake tokens on a specific outcome in a market
    pub fn stake_prediction(
        ctx: Context<StakePrediction>,
        outcome_index: u8,
        amount: u64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let prediction = &mut ctx.accounts.prediction;
        let clock = &ctx.accounts.clock;

        // Enforce market is active and not past deadline
        require!(!market.resolved, ForesightError::MarketAlreadyResolved);
        require!(
            clock.unix_timestamp < market.deadline,
            ForesightError::MarketDeadlinePassed
        );

        // Validate outcome index is valid
        require!(
            outcome_index < market.outcomes.len() as u8,
            ForesightError::InvalidOutcomeIndex
        );

        // Initialize prediction data
        prediction.user = ctx.accounts.user.key();
        prediction.market = market.key();
        prediction.outcome_index = outcome_index;
        prediction.amount = amount;
        prediction.claimed = false;
        prediction.bump = ctx.bumps.prediction;

        // Transfer USDC from user to vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.market_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // Update market stats
        market.total_pool = market.total_pool.checked_add(amount)
            .ok_or(ForesightError::CalculationError)?;
        market.total_predictions = market.total_predictions.checked_add(1)
            .ok_or(ForesightError::CalculationError)?;

        Ok(())
    }

    /// Resolve a market by setting the winning outcome
    pub fn resolve_market(ctx: Context<ResolveMarket>, winning_outcome: u8) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let creator_profile = &mut ctx.accounts.creator_profile;
        let clock = &ctx.accounts.clock;

        // Enforce market can be resolved
        require!(
            !market.resolved,
            ForesightError::MarketAlreadyResolved
        );
        
        require!(
            clock.unix_timestamp >= market.deadline,
            ForesightError::MarketDeadlineNotPassed
        );

        // Validate winning outcome index
        require!(
            winning_outcome < market.outcomes.len() as u8,
            ForesightError::InvalidOutcomeIndex
        );

        // Set market as resolved with winning outcome
        market.resolved = true;
        market.winning_outcome = Some(winning_outcome);

        // Update creator stats - will check if market meets criteria for "successful"
        Ok(())
    }

    /// Update creator stats after market resolution
    pub fn update_creator_stats(
        ctx: Context<UpdateCreatorStats>,
        unique_predictors: u32,
        total_staked: u64,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        let creator_profile = &mut ctx.accounts.creator_profile;

        // Update total volume
        creator_profile.total_volume = creator_profile.total_volume.checked_add(market.total_pool)
            .ok_or(ForesightError::CalculationError)?;

        // Track avg_ai_score using weighted moving average
        let market_count = creator_profile.markets_created as f32;
        creator_profile.avg_ai_score = ((creator_profile.avg_ai_score * (market_count - 1.0)) + market.ai_score) / market_count;

        // Increment successful markets if criteria met:
        // - At least 10 unique predictors
        // - At least 100 USDC staked
        // - AI score >= 0.7
        if unique_predictors >= 10 && total_staked >= 100_000_000 && market.ai_score >= 0.7 {
            creator_profile.successful_markets = creator_profile.successful_markets.checked_add(1)
                .ok_or(ForesightError::CalculationError)?;
                
            // Check for tier upgrade
            // Tier 0 -> Tier 1: 5 successful markets
            // Tier 1 -> Tier 2: 15 successful markets
            // Tier 2 -> Tier 3: 30 successful markets
            if creator_profile.tier == 0 && creator_profile.successful_markets >= 5 {
                creator_profile.tier = 1;
            } else if creator_profile.tier == 1 && creator_profile.successful_markets >= 15 {
                creator_profile.tier = 2;
            } else if creator_profile.tier == 2 && creator_profile.successful_markets >= 30 {
                creator_profile.tier = 3;
            }
        }

        Ok(())
    }

    /// Claim rewards for a winning prediction
    pub fn claim_reward(ctx: Context<ClaimReward>) -> Result<()> {
        let market = &ctx.accounts.market;
        let prediction = &mut ctx.accounts.prediction;
        
        // Enforce market is resolved
        require!(
            market.resolved,
            ForesightError::MarketNotResolved
        );
        
        // Check if user predicted winning outcome
        let winning_outcome = market.winning_outcome.ok_or(ForesightError::MarketNotResolved)?;
        require!(
            prediction.outcome_index == winning_outcome,
            ForesightError::NotWinner
        );
        
        // Check if already claimed
        require!(
            !prediction.claimed,
            ForesightError::RewardAlreadyClaimed
        );

        // Calculate payout:
        // reward = (user.amount / total_correct_stake) * (total_pool - creator_fee - protocol_fee)
        
        // For now we use a simplified calculation (actual implementation would require additional state tracking)
        let creator_fee = (market.total_pool)
            .checked_mul(market.creator_fee_bps as u64)
            .ok_or(ForesightError::CalculationError)?
            .checked_div(10000)
            .ok_or(ForesightError::CalculationError)?;
            
        let protocol_fee = market.total_pool
            .checked_mul(market.protocol_fee_bps as u64)
            .ok_or(ForesightError::CalculationError)?
            .checked_div(10000)
            .ok_or(ForesightError::CalculationError)?;
            
        // This is simplified - in a real implementation, you'd track total stake for each outcome
        // Here we just distribute the whole pool minus fees to the user
        let payout_amount = market.total_pool
            .checked_sub(creator_fee)
            .ok_or(ForesightError::CalculationError)?
            .checked_sub(protocol_fee)
            .ok_or(ForesightError::CalculationError)?;
            
        // Transfer from vault to user
        let seeds = &[
            b"market".as_ref(),
            market.creator.as_ref(),
            &[market.bump],
        ];
        let signer = &[&seeds[..]];
        
        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.market_vault.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_ctx, payout_amount)?;
        
        // Mark prediction as claimed
        prediction.claimed = true;
        
        Ok(())
    }

    /// Create a new creator profile
    pub fn create_creator_profile(ctx: Context<CreateCreatorProfile>) -> Result<()> {
        let creator_profile = &mut ctx.accounts.creator_profile;
        
        creator_profile.creator = ctx.accounts.creator.key();
        creator_profile.last_created_at = 0;
        creator_profile.markets_created = 0;
        creator_profile.successful_markets = 0;
        creator_profile.total_volume = 0;
        creator_profile.avg_ai_score = 0.0;
        creator_profile.tier = 0;
        creator_profile.bump = ctx.bumps.creator_profile;
        
        Ok(())
    }

    /// Close a market (admin only)
    pub fn close_market(ctx: Context<CloseMarket>) -> Result<()> {
        // No additional logic needed, the account will be closed and lamports returned
        // The access control is handled in the CloseMarket struct
        Ok(())
    }

    /// Set protocol configuration (admin only)
    pub fn set_protocol_config(
        ctx: Context<SetProtocolConfig>,
        protocol_fee_bps: u16,
        min_ai_score: f32,
        new_authority: Option<Pubkey>,
    ) -> Result<()> {
        let config = &mut ctx.accounts.protocol_config;
        
        // Update protocol fee (max 10%)
        require!(protocol_fee_bps <= 1000, ForesightError::CalculationError);
        config.protocol_fee_bps = protocol_fee_bps;
        
        // Update min AI score (between 0 and 1)
        require!(min_ai_score >= 0.0 && min_ai_score <= 1.0, ForesightError::CalculationError);
        config.min_ai_score = min_ai_score;
        
        // Update authority if provided
        if let Some(authority) = new_authority {
            config.authority = authority;
        }
        
        Ok(())
    }

    /// Initialize protocol config with default values
    pub fn initialize_protocol_config(ctx: Context<InitializeProtocolConfig>) -> Result<()> {
        let config = &mut ctx.accounts.protocol_config;
        
        config.authority = ctx.accounts.authority.key();
        config.protocol_fee_bps = 200; // 2% default
        config.min_ai_score = 0.5; // Default minimum score
        config.bump = ctx.bumps.protocol_config;
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(question: String, outcomes: Vec<String>, deadline: i64, creator_fee_bps: u16, ai_score: f32)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    #[account(
        init,
        payer = creator,
        space = 8 + // discriminator
                4 + question.len() + // question
                4 + outcomes.iter().map(|s| 4 + s.len()).sum::<usize>() + // outcomes
                8 + // deadline
                8 + // created_at
                1 + // resolved
                1 + 1 + // winning_outcome (Option<u8>)
                8 + // total_pool
                4 + // total_predictions
                4 + // ai_score (f32)
                2 + // creator_fee_bps
                2 + // protocol_fee_bps
                1 + // bump
                20, // buffer
        seeds = [
            b"market", 
            creator.key().as_ref(),
            &clock.unix_timestamp.to_le_bytes()
        ],
        bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"creator_profile", creator.key().as_ref()],
        bump = creator_profile.bump
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
    
    #[account(
        seeds = [b"protocol_config"],
        bump = protocol_config.bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    pub clock: Sysvar<'info, Clock>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(outcome_index: u8, amount: u64)]
pub struct StakePrediction<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        init,
        payer = user,
        space = 8 + // discriminator
                32 + // user
                32 + // market
                1 + // outcome_index
                8 + // amount
                1 + // claimed
                1 + // bump
                16, // buffer
        seeds = [
            b"prediction", 
            market.key().as_ref(),
            user.key().as_ref()
        ],
        bump,
        constraint = market.deadline > clock.unix_timestamp @ ForesightError::MarketDeadlinePassed
    )]
    pub prediction: Account<'info, Prediction>,
    
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ ProgramError::InvalidArgument
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    pub clock: Sysvar<'info, Clock>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(winning_outcome: u8)]
pub struct ResolveMarket<'info> {
    #[account(
        mut,
        constraint = creator.key() == market.creator @ ForesightError::NotMarketCreator
    )]
    pub creator: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"market", creator.key().as_ref(), &market.created_at.to_le_bytes()],
        bump = market.bump,
        constraint = !market.resolved @ ForesightError::MarketAlreadyResolved,
        constraint = clock.unix_timestamp >= market.deadline @ ForesightError::MarketDeadlineNotPassed
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"creator_profile", creator.key().as_ref()],
        bump = creator_profile.bump
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
    
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
#[instruction(unique_predictors: u32, total_staked: u64)]
pub struct UpdateCreatorStats<'info> {
    #[account(
        mut,
        constraint = creator.key() == market.creator @ ForesightError::NotMarketCreator
    )]
    pub creator: Signer<'info>,
    
    #[account(
        seeds = [b"market", creator.key().as_ref(), &market.created_at.to_le_bytes()],
        bump = market.bump,
        constraint = market.resolved @ ForesightError::MarketNotResolved
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"creator_profile", creator.key().as_ref()],
        bump = creator_profile.bump
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
}

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        seeds = [b"market", market.creator.as_ref(), &market.created_at.to_le_bytes()],
        bump = market.bump,
        constraint = market.resolved @ ForesightError::MarketNotResolved
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"prediction", market.key().as_ref(), user.key().as_ref()],
        bump = prediction.bump,
        constraint = prediction.user == user.key() @ ProgramError::InvalidArgument,
        constraint = !prediction.claimed @ ForesightError::RewardAlreadyClaimed
    )]
    pub prediction: Account<'info, Prediction>,
    
    #[account(
        mut,
        seeds = [b"vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ ProgramError::InvalidArgument
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CreateCreatorProfile<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    #[account(
        init,
        payer = creator,
        space = 8 + // discriminator
                32 + // creator
                8 + // last_created_at
                4 + // markets_created
                4 + // successful_markets
                8 + // total_volume
                4 + // avg_ai_score (f32)
                1 + // tier
                1 + // bump
                32, // buffer
        seeds = [b"creator_profile", creator.key().as_ref()],
        bump
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseMarket<'info> {
    #[account(
        mut,
        address = protocol_config.authority @ ForesightError::NotProtocolAuthority
    )]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        close = authority,
        constraint = market.resolved || clock.unix_timestamp >= market.deadline @ ForesightError::MarketNotResolved
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        seeds = [b"protocol_config"],
        bump = protocol_config.bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct SetProtocolConfig<'info> {
    #[account(
        mut,
        address = protocol_config.authority @ ForesightError::NotProtocolAuthority
    )]
    pub authority: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"protocol_config"],
        bump = protocol_config.bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
}

#[derive(Accounts)]
pub struct InitializeProtocolConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = 8 + // discriminator
                32 + // authority
                2 + // protocol_fee_bps
                4 + // min_ai_score
                1 + // bump
                16, // buffer
        seeds = [b"protocol_config"],
        bump
    )]
    pub protocol_config: Account<'info, ProtocolConfig>,
    
    pub system_program: Program<'info, System>,
}

/// Market account to store prediction market data
#[account]
pub struct Market {
    pub creator: Pubkey,
    pub question: String,
    pub outcomes: Vec<String>,
    pub deadline: i64,
    pub created_at: i64, // Added for PDA seed
    pub end_time: i64,
    pub resolved: bool,
    pub winning_outcome: Option<u8>,
    pub total_pool: u64,
    pub total_predictions: u32,
    pub ai_score: f32,
    pub creator_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub bump: u8,
}

/// Prediction account to track user stakes
#[account]
pub struct Prediction {
    pub user: Pubkey,
    pub market: Pubkey,
    pub outcome_index: u8,
    pub amount: u64,
    pub claimed: bool,
    pub bump: u8,
}

/// Creator profile to track reputation and manage cooldown
#[account]
pub struct CreatorProfile {
    pub creator: Pubkey,
    pub last_created_at: i64,
    pub markets_created: u32,
    pub successful_markets: u32,
    pub total_volume: u64,
    pub avg_ai_score: f32,
    pub tier: u8, // 0 = New, 1 = Trusted, 2 = Featured, 3 = Verified
    pub bump: u8,
}

/// Protocol configuration for global parameters
#[account]
pub struct ProtocolConfig {
    pub authority: Pubkey,
    pub protocol_fee_bps: u16,
    pub min_ai_score: f32,
    pub bump: u8,
}
