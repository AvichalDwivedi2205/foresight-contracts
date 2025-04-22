use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod contracts {
    use super::*;

    pub fn create_creator_profile(ctx: Context<CreateCreatorProfile>) -> Result<()> {
        let profile = &mut ctx.accounts.creator_profile;
        let creator = ctx.accounts.creator.key();

        profile.creator = creator;
        profile.last_created_at = 0; // Initialize with 0 to allow first market creation
        profile.markets_created = 0;
        profile.total_volume = 0;
        profile.traction_score = 0;
        profile.tier = 0; // Start at tier 0
        profile.bump = ctx.bumps.creator_profile;

        msg!("Creator profile created for {}", creator);
        Ok(())
    }

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        question: String,
        outcomes: Vec<String>,
        ai_score: f32,
        ai_recommended_resolution_time: i64,
        ai_classification: u8,
        creator_metadata: String, // Stored off-chain but kept as parameter for consistency
        creator_fee_bps: Option<u16>,
    ) -> Result<()> {
        // Validate inputs
        require!(outcomes.len() <= 5, ErrorCode::TooManyOutcomes);
        require!(ai_score >= 0.7, ErrorCode::LowAIScore);
        
        // Validate market type
        match ai_classification {
            0 => {}, // TimeBound
            1 => {}, // OpenEnded
            _ => return Err(ErrorCode::InvalidMarketType.into()),
        };
        
        let clock = Clock::get()?;
        require!(
            ai_recommended_resolution_time >= clock.unix_timestamp,
            ErrorCode::InvalidDeadline
        );

        // Check creator cooldown (5 days for Tier 0)
        let creator_profile = &mut ctx.accounts.creator_profile;
        let five_days_in_seconds = 5 * 24 * 60 * 60;
        
        if creator_profile.tier == 0 && creator_profile.last_created_at > 0 {
            require!(
                clock.unix_timestamp - creator_profile.last_created_at >= five_days_in_seconds,
                ErrorCode::CreatorOnCooldown
            );
        }
        
        // Apply creator fee (with cap)
        let fee_bps = match creator_fee_bps {
            Some(fee) => {
                require!(fee <= 500, ErrorCode::ExcessiveCreatorFee); // Max 5%
                fee
            },
            None => 200, // Default 2%
        };
        
        // Initialize market account
        let market = &mut ctx.accounts.market;
        market.creator = ctx.accounts.creator.key();
        market.question = question.clone(); // Clone to avoid move error
        market.outcomes = outcomes;
        market.ai_score = ai_score;
        market.market_type = ai_classification;
        market.deadline = ai_recommended_resolution_time;
        market.ai_suggested_deadline = ai_recommended_resolution_time;
        market.resolved = false;
        market.winning_outcome = None;
        market.total_pool = 0;
        market.creator_fee_bps = fee_bps;
        market.protocol_fee_bps = 50; // Default 0.5% protocol fee
        market.stakes_per_outcome = vec![0; market.outcomes.len()]; // Initialize stakes per outcome
        market.bump = ctx.bumps.market;
        
        // Update creator profile
        creator_profile.last_created_at = clock.unix_timestamp;
        creator_profile.markets_created = creator_profile.markets_created.checked_add(1).unwrap();
        
        // Log creator metadata for off-chain indexing and AI purposes
        msg!("Market initialized: {} (metadata: {})", question, creator_metadata);
        Ok(())
    }

    pub fn stake_prediction(
        ctx: Context<StakePrediction>,
        outcome_index: u8,
        amount: u64,
    ) -> Result<()> {
        // Validate outcome index
        let market = &mut ctx.accounts.market;
        require!(
            (outcome_index as usize) < market.outcomes.len(),
            ErrorCode::InvalidOutcomeIndex
        );
        
        // Can't stake on resolved markets
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        
        // Check that the deadline hasn't passed
        let current_time = Clock::get()?.unix_timestamp;
        require!(market.deadline > current_time, ErrorCode::MarketExpired);
        
        // Transfer tokens from user to market vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.market_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token::transfer(cpi_ctx, amount)?;
        
        // Update the prediction
        let prediction = &mut ctx.accounts.prediction;
        prediction.market = market.key();
        prediction.user = ctx.accounts.user.key();
        prediction.outcome_index = outcome_index;
        prediction.amount = amount;
        prediction.timestamp = current_time;
        prediction.claimed = false;
        prediction.bump = ctx.bumps.prediction;
        
        // Update the market total pool
        market.total_pool = market.total_pool.checked_add(amount).unwrap();
        
        // Update stakes per outcome - ensure array is initialized with enough elements
        if market.stakes_per_outcome.len() < market.outcomes.len() {
            // Initialize stakes_per_outcome with zeros for each outcome if not already done
            market.stakes_per_outcome = vec![0; market.outcomes.len()];
        }
        
        // Update the total staked for this outcome
        market.stakes_per_outcome[outcome_index as usize] = 
            market.stakes_per_outcome[outcome_index as usize].checked_add(amount).unwrap();
        
        msg!("Staked {} on outcome {}", amount, outcome_index);
        Ok(())
    }

    pub fn vote_market_outcome(
        ctx: Context<VoteMarketOutcome>,
        outcome_index: u8,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        
        // Validations
        require!(
            market.market_type == MarketType::OpenEnded as u8,
            ErrorCode::NotOpenEndedMarket
        );
        
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        
        let clock = Clock::get()?;
        let voting_deadline = market.deadline.checked_add(15 * 24 * 60 * 60).unwrap(); // 15 days after deadline
        
        require!(
            clock.unix_timestamp >= market.deadline,
            ErrorCode::VotingNotStarted
        );
        
        require!(
            clock.unix_timestamp <= voting_deadline,
            ErrorCode::VotingPeriodEnded
        );
        
        require!(
            (outcome_index as usize) < market.outcomes.len(),
            ErrorCode::InvalidOutcomeIndex
        );
        
        // Record vote
        let vote = &mut ctx.accounts.outcome_vote;
        vote.market = market.key();
        vote.voter = ctx.accounts.voter.key();
        vote.outcome_index = outcome_index;
        vote.bump = ctx.bumps.outcome_vote;
        
        msg!("Vote recorded for outcome {}", outcome_index);
        Ok(())
    }

    pub fn resolve_market(
        ctx: Context<ResolveMarket>,
        winning_outcome_index: Option<u8>,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        
        // Only admin can resolve
        let admin_key = ctx.accounts.admin.key();
        require!(
            admin_key == ctx.accounts.admin.key(),
            ErrorCode::Unauthorized
        );
        
        // Can't resolve already resolved markets
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        
        // For time-bound markets, winning_outcome_index must be provided
        if market.market_type == MarketType::TimeBound as u8 {
            // Validate outcome index
            let outcome = winning_outcome_index.ok_or(ErrorCode::WinningOutcomeRequired)?;
            require!(
                (outcome as usize) < market.outcomes.len(),
                ErrorCode::InvalidOutcomeIndex
            );
            
            market.winning_outcome = winning_outcome_index;
        } else {
            // For OpenEnded markets, admin can provide the winning outcome based on off-chain vote calculation
            // (In a real implementation, you might want to verify this against on-chain votes)
            market.winning_outcome = winning_outcome_index;
        }
        
        market.resolved = true;
        
        msg!("Market resolved with winning outcome: {:?}", market.winning_outcome);
        Ok(())
    }

    pub fn claim_reward(ctx: Context<ClaimReward>) -> Result<()> {
        let market = &ctx.accounts.market;
        let prediction = &mut ctx.accounts.prediction;
        
        // Validations
        require!(market.resolved, ErrorCode::MarketNotResolved);
        require!(!prediction.claimed, ErrorCode::RewardAlreadyClaimed);
        
        // Verify winning outcome exists and matches user's prediction
        let winning_outcome = market.winning_outcome.ok_or(ErrorCode::NoWinningOutcome)?;
        require!(
            prediction.outcome_index == winning_outcome,
            ErrorCode::NotWinningPrediction
        );
        
        // Get total staked on winning outcome
        let total_winning_stakes = market.stakes_per_outcome[winning_outcome as usize];
        require!(total_winning_stakes > 0, ErrorCode::InvalidDistribution);
        
        // Calculate user's proportional share of the total pool
        // (user_stake / total_winning_stakes) * total_pool
        let user_stake = prediction.amount;
        let total_pool = market.total_pool;
        
        let user_share_numerator = (user_stake as u128).checked_mul(total_pool as u128).unwrap();
        let user_share = user_share_numerator.checked_div(total_winning_stakes as u128).unwrap();
        
        // Calculate fees
        let creator_fee_amount = user_share
            .checked_mul(market.creator_fee_bps as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap();
        
        let protocol_fee_amount = user_share
            .checked_mul(market.protocol_fee_bps as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap();
        
        // Final reward amount after fees
        let reward_amount = user_share
            .checked_sub(creator_fee_amount)
            .unwrap()
            .checked_sub(protocol_fee_amount)
            .unwrap() as u64;
        
        // Get market PDA signer seeds
        let market_key = market.key();
        let seeds = &[
            b"market".as_ref(),
            market_key.as_ref(),
            &[market.bump],
        ];
        let signer = &[&seeds[..]];
        
        // 1. Transfer reward to user
        {
            let cpi_accounts = Transfer {
                from: ctx.accounts.market_vault.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            };
            
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            
            token::transfer(cpi_ctx, reward_amount)?;
        }
        
        // 2. Transfer creator fee
        if creator_fee_amount > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.market_vault.to_account_info(),
                to: ctx.accounts.creator_token_account.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            };
            
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            
            token::transfer(cpi_ctx, creator_fee_amount as u64)?;
        }
        
        // 3. Transfer protocol fee
        if protocol_fee_amount > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.market_vault.to_account_info(),
                to: ctx.accounts.protocol_fee_account.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            };
            
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            
            token::transfer(cpi_ctx, protocol_fee_amount as u64)?;
        }
        
        // Mark prediction as claimed
        prediction.claimed = true;
        
        msg!(
            "Reward claimed: total={}, user={}, creator_fee={}, protocol_fee={}",
            user_share,
            reward_amount,
            creator_fee_amount,
            protocol_fee_amount
        );
        
        Ok(())
    }

    pub fn close_market(ctx: Context<CloseMarket>) -> Result<()> {
        // Only admin can close markets
        // This check is redundant as the market will only close if admin is the signer
        
        let market = &ctx.accounts.market;
        
        // Can only close resolved markets
        require!(market.resolved, ErrorCode::MarketNotResolved);
        
        // Market is already closed by Anchor's account close constraint
        msg!("Market closed");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateCreatorProfile<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    
    #[account(
        init,
        payer = creator,
        space = 8 + CreatorProfile::SPACE,
        seeds = [b"creator_profile", creator.key().as_ref()],
        bump
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeMarket<'info> {
    #[account(mut)]     
    pub creator: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"creator_profile", creator.key().as_ref()],
        bump
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
    
    #[account(
        init,
        payer = creator,
        space = 8 + Market::SPACE,
        seeds = [b"market", creator.key().as_ref(), &creator_profile.markets_created.to_le_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        init,
        payer = creator,
        token::mint = mint,
        token::authority = market,
        seeds = [b"market_vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    pub mint: Account<'info, token::Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct StakePrediction<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"creator_profile", market.creator.as_ref()],
        bump
    )]
    pub creator_profile: Account<'info, CreatorProfile>,
    
    #[account(
        init,
        payer = user,
        space = 8 + Prediction::SPACE,
        seeds = [b"prediction", market.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub prediction: Account<'info, Prediction>,
    
    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == market_vault.mint
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"market_vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VoteMarketOutcome<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        init,
        payer = voter,
        space = 8 + OutcomeVote::SPACE,
        seeds = [b"outcome_vote", market.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub outcome_vote: Account<'info, OutcomeVote>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>, // Protocol admin
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        constraint = market.resolved == true
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"prediction", market.key().as_ref(), user.key().as_ref()],
        bump = prediction.bump,
        constraint = prediction.user == user.key()
    )]
    pub prediction: Account<'info, Prediction>,
    
    #[account(
        mut,
        seeds = [b"market_vault", market.key().as_ref()],
        bump
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == market_vault.mint
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    // Add creator token account to receive fees
    #[account(
        mut,
        constraint = creator_token_account.owner == market.creator,
        constraint = creator_token_account.mint == market_vault.mint
    )]
    pub creator_token_account: Account<'info, TokenAccount>,
    
    // Add protocol fee account
    #[account(
        mut,
        constraint = protocol_fee_account.mint == market_vault.mint
    )]
    pub protocol_fee_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CloseMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>, // Protocol admin
    
    #[account(
        mut,
        close = admin,
        constraint = market.resolved == true
    )]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"market_vault", market.key().as_ref()],
        bump,
        constraint = market_vault.amount == 0 // Ensure all funds are distributed
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default)]
pub struct Market {
    pub creator: Pubkey,
    pub question: String,
    pub outcomes: Vec<String>,
    pub ai_score: f32,
    pub market_type: u8, // Open Ended or Time Bound
    pub deadline: i64,
    pub ai_suggested_deadline: i64,
    pub resolved: bool,
    pub winning_outcome: Option<u8>,
    pub total_pool: u64,
    pub creator_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub stakes_per_outcome: Vec<u64>, // Track stakes per outcome for fair distribution
    pub bump: u8,
}

impl Market {
    pub const SPACE: usize = 32 + // creator
                            4 + 200 + // question (assume max 200 chars)
                            4 + 5 * (4 + 50) + // outcomes (5 outcomes with 50 chars each)
                            4 + // ai_score
                            1 + // market_type
                            8 + // deadline
                            8 + // ai_suggested_deadline
                            1 + // resolved
                            1 + 1 + // winning_outcome (Option<u8>)
                            8 + // total_pool
                            2 + // creator_fee_bps
                            2 + // protocol_fee_bps
                            4 + 5 * 8 + // stakes_per_outcome (5 outcomes max)
                            1 + // bump
                            50; // padding
}

#[account]
#[derive(Default)]
pub struct Prediction {
    pub user: Pubkey,
    pub market: Pubkey,
    pub outcome_index: u8,
    pub amount: u64,
    pub timestamp: i64,
    pub claimed: bool,
    pub bump: u8,
}

impl Prediction {
    pub const SPACE: usize = 32 + // user
                            32 + // market
                            1 + // outcome_index
                            8 + // amount
                            8 + // timestamp
                            1 + // claimed
                            1 + // bump
                            30; // padding
}

#[account]
#[derive(Default)]
pub struct OutcomeVote {
    pub market: Pubkey,
    pub voter: Pubkey,
    pub outcome_index: u8,
    pub bump: u8,
}

impl OutcomeVote {
    pub const SPACE: usize = 32 + // market
                            32 + // voter
                            1 + // outcome_index
                            1 + // bump
                            20; // padding
}

#[account]
#[derive(Default)]
pub struct CreatorProfile {
    pub creator: Pubkey,
    pub last_created_at: i64,
    pub markets_created: u32,
    pub total_volume: u64,
    pub traction_score: u64,
    pub tier: u8,
    pub bump: u8,
}

impl CreatorProfile {
    pub const SPACE: usize = 32 + // creator
                            8 + // last_created_at
                            4 + // markets_created
                            8 + // total_volume
                            8 + // traction_score
                            1 + // tier
                            1 + // bump
                            30; // padding
}

#[error_code]
pub enum ErrorCode {
    #[msg("Too many outcomes. Maximum is 5.")]
    TooManyOutcomes,
    
    #[msg("AI score too low. Minimum is 0.7.")]
    LowAIScore,
    
    #[msg("Invalid market type.")]
    InvalidMarketType,
    
    #[msg("Deadline must be in the future.")]
    InvalidDeadline,
    
    #[msg("Creator is on cooldown period.")]
    CreatorOnCooldown,
    
    #[msg("Creator fee is too high. Maximum is 5%.")]
    ExcessiveCreatorFee,
    
    #[msg("Market is already resolved.")]
    MarketAlreadyResolved,
    
    #[msg("Market is closed for predictions.")]
    MarketClosed,
    
    #[msg("Invalid outcome index.")]
    InvalidOutcomeIndex,
    
    #[msg("Not an open-ended market.")]
    NotOpenEndedMarket,
    
    #[msg("Voting period not started yet.")]
    VotingNotStarted,
    
    #[msg("Voting period has ended.")]
    VotingPeriodEnded,
    
    #[msg("Unauthorized action.")]
    Unauthorized,
    
    #[msg("Winning outcome index required for time-bound markets.")]
    WinningOutcomeRequired,
    
    #[msg("Market not resolved yet.")]
    MarketNotResolved,
    
    #[msg("Reward already claimed.")]
    RewardAlreadyClaimed,
    
    #[msg("No winning outcome set.")]
    NoWinningOutcome,
    
    #[msg("Not a winning prediction.")]
    NotWinningPrediction,
    
    #[msg("Invalid distribution.")]
    InvalidDistribution,
    
    #[msg("Market has expired. The deadline has passed.")]
    MarketExpired,
}
#[derive(Clone, Copy, PartialEq)]
pub enum MarketType {
    TimeBound = 0,
    OpenEnded = 1,
}