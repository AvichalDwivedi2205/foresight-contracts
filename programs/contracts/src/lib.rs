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
        profile.last_created_at = 0;
        profile.markets_created = 0;
        profile.total_volume = 0;
        profile.traction_score = 0;
        profile.tier = 0;
        profile.bump = ctx.bumps.creator_profile;

        msg!("Creator profile created for {}", creator);
        Ok(())
    }

    pub fn initialize_ai_resolver(
        ctx: Context<InitializeAIResolver>,
    ) -> Result<()> {
        let resolver = &mut ctx.accounts.ai_resolver;
        resolver.authority = ctx.accounts.admin.key();
        resolver.active = true;
        resolver.resolution_count = 0;
        resolver.bump = ctx.bumps.ai_resolver;
        
        msg!("AI resolver initialized with authority: {}", resolver.authority);
        Ok(())
    }

    pub fn resolve_market_via_ai(
        ctx: Context<ResolveMarketViaAI>,
        winning_outcome_index: u8,
        ai_confidence_score: f32,
        resolution_data: String,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        
        require!(
            ctx.accounts.ai_resolver.authority == ctx.accounts.resolver_authority.key(),
            ErrorCode::Unauthorized
        );
        
        require!(ctx.accounts.ai_resolver.active, ErrorCode::ResolverInactive);
        
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        
        let current_time = Clock::get()?.unix_timestamp;
        require!(current_time >= market.deadline, ErrorCode::MarketNotExpired);
        
        require!(ai_confidence_score >= 0.85, ErrorCode::LowAIConfidence);
        
        require!(
            market.market_type == MarketType::TimeBound as u8,
            ErrorCode::NotTimeBoundMarket
        );
        
        require!(
            (winning_outcome_index as usize) < market.outcomes.len(),
            ErrorCode::InvalidOutcomeIndex
        );

        market.winning_outcome = Some(winning_outcome_index);
        market.resolved = true;
        
        ctx.accounts.ai_resolver.resolution_count = ctx.accounts.ai_resolver.resolution_count.checked_add(1).unwrap();
        
        msg!("Market resolved by AI with outcome: {}, confidence: {}", winning_outcome_index, ai_confidence_score);
        msg!("Resolution data: {}", resolution_data);
        
        Ok(())
    }

    pub fn create_market(
        ctx: Context<InitializeMarket>,
        question: String,
        outcomes: Vec<String>,
        ai_score: f32,
        ai_recommended_resolution_time: i64,
        ai_classification: u8,
        creator_metadata: String,
        creator_fee_bps: Option<u16>,
        ai_resolvable: Option<bool>, 
    ) -> Result<()> {
        require!(outcomes.len() <= 5, ErrorCode::TooManyOutcomes);
        require!(ai_score >= 0.7, ErrorCode::LowAIScore);
        
        match ai_classification {
            0 => {}, 
            1 => {}, 
            _ => return Err(ErrorCode::InvalidMarketType.into()),
        };
        
        let clock = Clock::get()?;
        require!(
            ai_recommended_resolution_time >= clock.unix_timestamp,
            ErrorCode::InvalidDeadline
        );

        let creator_profile = &mut ctx.accounts.creator_profile;
        let five_days_in_seconds = 5 * 24 * 60 * 60;
        
        if creator_profile.tier == 0 && creator_profile.last_created_at > 0 {
            require!(
                clock.unix_timestamp - creator_profile.last_created_at >= five_days_in_seconds,
                ErrorCode::CreatorOnCooldown
            );
        }
        
        let fee_bps = match creator_fee_bps {
            Some(fee) => {
                require!(fee <= 500, ErrorCode::ExcessiveCreatorFee); 
                fee
            },
            None => 200, // Default 2%
        };
        
        // Initialize market account
        let market = &mut ctx.accounts.market;
        market.creator = ctx.accounts.creator.key();
        market.question = question.clone();
        market.outcomes = outcomes;
        market.ai_score = ai_score;
        market.market_type = ai_classification;
        market.deadline = ai_recommended_resolution_time;
        market.ai_suggested_deadline = ai_recommended_resolution_time;
        market.resolved = false;
        market.winning_outcome = None;
        market.total_pool = 0;
        market.creator_fee_bps = fee_bps;
        market.protocol_fee_bps = 50; 
        market.stakes_per_outcome = vec![0; market.outcomes.len()]; 
        market.ai_resolvable = ai_resolvable.unwrap_or(true); 
        market.bump = ctx.bumps.market;
        
        creator_profile.last_created_at = clock.unix_timestamp;
        creator_profile.markets_created = creator_profile.markets_created.checked_add(1).unwrap();

        msg!("Market initialized: {} (metadata: {})", question, creator_metadata);
        Ok(())
    }

    pub fn stake_prediction(
        ctx: Context<StakePrediction>,
        outcome_index: u8,
        amount: u64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        require!(
            (outcome_index as usize) < market.outcomes.len(),
            ErrorCode::InvalidOutcomeIndex
        );
        
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        
        let current_time = Clock::get()?.unix_timestamp;
        require!(market.deadline > current_time, ErrorCode::MarketExpired);

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
        
        let market = &ctx.accounts.market;
        
        require!(market.resolved, ErrorCode::MarketNotResolved);
        
        msg!("Market closed");
        Ok(())
    }

    pub fn register_vote_authority(
        ctx: Context<RegisterVoteAuthority>,
        weight: u8,
    ) -> Result<()> {
        let _admin_key = ctx.accounts.admin.key();
        
        require!(weight >= 1 && weight <= 5, ErrorCode::InvalidWeight);
        
        require!(
            ctx.accounts.market.market_type == MarketType::OpenEnded as u8,
            ErrorCode::NotOpenEndedMarket
        );
        
        // Initialize vote authority
        let authority = &mut ctx.accounts.vote_authority;
        authority.market = ctx.accounts.market.key();
        authority.authority = ctx.accounts.authority.key();
        authority.weight = weight;
        authority.has_voted = false;
        authority.vote = None;
        authority.bump = ctx.bumps.vote_authority;
        
        msg!("Vote authority registered with weight {}", weight);
        Ok(())
    }

    pub fn initialize_vote_result(
        ctx: Context<InitializeVoteResult>,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        
        require!(
            market.market_type == MarketType::OpenEnded as u8,
            ErrorCode::NotOpenEndedMarket
        );
        
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp >= market.deadline,
            ErrorCode::VotingNotStarted
        );
        
        let vote_result = &mut ctx.accounts.vote_result;
        vote_result.market = market.key();
        vote_result.vote_tallies = vec![0; market.outcomes.len()];
        vote_result.stake_weights = vec![0; market.outcomes.len()];
        vote_result.vote_count = 0;
        vote_result.resolution_proposed = false;
        vote_result.proposed_outcome = None;
        vote_result.proposal_time = 0;
        vote_result.challenge_count = 0;
        vote_result.finalized = false;
        vote_result.bump = ctx.bumps.vote_result;
        
        msg!("Vote result initialized for market {}", market.key());
        Ok(())
    }

    pub fn stake_weighted_vote(
        ctx: Context<StakeWeightedVote>,
        outcome_index: u8,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        let voter = &ctx.accounts.voter;
        let prediction = &ctx.accounts.prediction;
        let vote_result = &mut ctx.accounts.vote_result;
        
        require!(
            market.market_type == MarketType::OpenEnded as u8,
            ErrorCode::NotOpenEndedMarket
        );
        
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        
        require!(
            prediction.user == voter.key(),
            ErrorCode::Unauthorized
        );
        
        require!(
            prediction.amount > 0,
            ErrorCode::InsufficientStake
        );
        
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
        
        let vote = &mut ctx.accounts.outcome_vote;
        vote.market = market.key();
        vote.voter = voter.key();
        vote.outcome_index = outcome_index;
        vote.bump = ctx.bumps.outcome_vote;
        
        vote_result.vote_tallies[outcome_index as usize] = 
            vote_result.vote_tallies[outcome_index as usize].checked_add(1).unwrap();
        
        vote_result.stake_weights[outcome_index as usize] = 
            vote_result.stake_weights[outcome_index as usize].checked_add(prediction.amount).unwrap();
        
        vote_result.vote_count = vote_result.vote_count.checked_add(1).unwrap();
        
        msg!("Stake-weighted vote recorded for outcome {} with weight {}", outcome_index, prediction.amount);
        Ok(())
    }

    pub fn propose_resolution(
        ctx: Context<ProposeResolution>,
        outcome_index: u8,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        let vote_result = &mut ctx.accounts.vote_result;
        let authority = &mut ctx.accounts.vote_authority;
        
        require!(
            market.market_type == MarketType::OpenEnded as u8,
            ErrorCode::NotOpenEndedMarket
        );
        
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        
        let clock = Clock::get()?;
        let voting_deadline = market.deadline.checked_add(15 * 24 * 60 * 60).unwrap(); // 15 days
        
        require!(
            clock.unix_timestamp > voting_deadline,
            ErrorCode::VotingNotEnded
        );
        
        require!(
            (outcome_index as usize) < market.outcomes.len(),
            ErrorCode::InvalidOutcomeIndex
        );
        
        authority.has_voted = true;
        authority.vote = Some(outcome_index);
        
        if !vote_result.resolution_proposed {
            vote_result.resolution_proposed = true;
            vote_result.proposed_outcome = Some(outcome_index);
            vote_result.proposal_time = clock.unix_timestamp;
            
            msg!("Resolution proposed with outcome {}", outcome_index);
        } else {
            if vote_result.proposed_outcome == Some(outcome_index) {
                msg!("Resolution proposal confirmed for outcome {}", outcome_index);
            } else {
                vote_result.challenge_count = vote_result.challenge_count.checked_add(1).unwrap();
                msg!("Resolution challenged with alternative outcome {}", outcome_index);
            }
        }
        
        Ok(())
    }

    pub fn finalize_resolution(
        ctx: Context<FinalizeResolution>,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let vote_result = &mut ctx.accounts.vote_result;
        
        require!(
            market.market_type == MarketType::OpenEnded as u8,
            ErrorCode::NotOpenEndedMarket
        );
        
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        require!(vote_result.resolution_proposed, ErrorCode::NoProposedResolution);
        
        let clock = Clock::get()?;
        let challenge_deadline = vote_result.proposal_time.checked_add(48 * 60 * 60).unwrap(); // 48 hours
        
        require!(
            clock.unix_timestamp > challenge_deadline,
            ErrorCode::ChallengePeriodActive
        );
        
        if vote_result.challenge_count > 0 {
            let mut max_stake = 0;
            let mut winning_index = 0;
            
            for (i, &stake) in vote_result.stake_weights.iter().enumerate() {
                if stake > max_stake {
                    max_stake = stake;
                    winning_index = i;
                }
            }
            
            vote_result.proposed_outcome = Some(winning_index as u8);
            msg!("Resolution determined by stake-weighted vote: outcome {}", winning_index);
        }
        
        market.winning_outcome = vote_result.proposed_outcome;
        market.resolved = true;
        vote_result.finalized = true;
        
        msg!("Market resolution finalized with outcome {:?}", market.winning_outcome);
        Ok(())
    }

    pub fn challenge_resolution(
        ctx: Context<ChallengeResolution>,
        evidence: String,
    ) -> Result<()> {
        let vote_result = &mut ctx.accounts.vote_result;
        
        require!(vote_result.resolution_proposed, ErrorCode::NoProposedResolution);
        require!(!vote_result.finalized, ErrorCode::ResolutionFinalized);
        
        let clock = Clock::get()?;
        let challenge_deadline = vote_result.proposal_time.checked_add(48 * 60 * 60).unwrap(); // 48 hours
        
        require!(
            clock.unix_timestamp <= challenge_deadline,
            ErrorCode::ChallengePeriodEnded
        );
        
        // Register challenge
        vote_result.challenge_count = vote_result.challenge_count.checked_add(1).unwrap();
        
        msg!("Resolution challenged with evidence: {}", evidence);
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
pub struct InitializeAIResolver<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        init,
        payer = admin,
        space = 8 + AIResolver::SPACE,
        seeds = [b"ai_resolver", admin.key().as_ref()],
        bump
    )]
    pub ai_resolver: Account<'info, AIResolver>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveMarketViaAI<'info> {
    #[account(mut)]
    pub resolver_authority: Signer<'info>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"ai_resolver", resolver_authority.key().as_ref()],
        bump
    )]
    pub ai_resolver: Account<'info, AIResolver>,
    
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

#[derive(Accounts)]
pub struct RegisterVoteAuthority<'info> {
    #[account(mut)]
    pub admin: Signer<'info>, // Protocol admin
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    /// The account that will be granted voting authority
    pub authority: SystemAccount<'info>,
    
    #[account(
        init,
        payer = admin,
        space = 8 + VoteAuthority::SPACE,
        seeds = [b"vote_authority", market.key().as_ref(), authority.key().as_ref()],
        bump
    )]
    pub vote_authority: Account<'info, VoteAuthority>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeVoteResult<'info> {
    #[account(mut)]
    pub admin: Signer<'info>, // Protocol admin
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        init,
        payer = admin,
        space = 8 + VoteResult::SPACE,
        seeds = [b"vote_result", market.key().as_ref()],
        bump
    )]
    pub vote_result: Account<'info, VoteResult>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StakeWeightedVote<'info> {
    #[account(mut)]
    pub voter: Signer<'info>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"prediction", market.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub prediction: Account<'info, Prediction>,
    
    #[account(
        mut,
        seeds = [b"vote_result", market.key().as_ref()],
        bump
    )]
    pub vote_result: Account<'info, VoteResult>,
    
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
pub struct ProposeResolution<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"vote_result", market.key().as_ref()],
        bump
    )]
    pub vote_result: Account<'info, VoteResult>,
    
    #[account(
        mut,
        seeds = [b"vote_authority", market.key().as_ref(), authority.key().as_ref()],
        bump
    )]
    pub vote_authority: Account<'info, VoteAuthority>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizeResolution<'info> {
    #[account(mut)]
    pub admin: Signer<'info>, // Protocol admin
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"vote_result", market.key().as_ref()],
        bump
    )]
    pub vote_result: Account<'info, VoteResult>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ChallengeResolution<'info> {
    #[account(mut)]
    pub challenger: Signer<'info>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(
        mut,
        seeds = [b"vote_result", market.key().as_ref()],
        bump
    )]
    pub vote_result: Account<'info, VoteResult>,
    
    pub system_program: Program<'info, System>,
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
    pub ai_resolvable: bool, // Flag indicating if this market can be resolved by AI
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
                            1 + // ai_resolvable
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

#[account]
pub struct AIResolver {
    pub authority: Pubkey,       // The authorized AI service public key
    pub active: bool,            // Whether this resolver is active
    pub resolution_count: u64,   // Number of markets resolved
    pub bump: u8,
}

impl AIResolver {
    pub const SPACE: usize = 32 + // authority
                            1 +  // active
                            8 +  // resolution_count
                            1 +  // bump
                            30;  // padding
}

// New structures for enhanced open-ended market resolution
#[account]
pub struct VoteResult {
    pub market: Pubkey,
    pub vote_tallies: Vec<u64>,    // Number of votes per outcome
    pub stake_weights: Vec<u64>,   // Stake-weighted votes per outcome
    pub vote_count: u64,           // Total number of votes
    pub resolution_proposed: bool, // Whether a resolution has been proposed
    pub proposed_outcome: Option<u8>, // Proposed winning outcome
    pub proposal_time: i64,        // When the resolution was proposed
    pub challenge_count: u8,       // Number of challenges to the proposal
    pub finalized: bool,           // Whether voting is finalized
    pub bump: u8,
}

impl VoteResult {
    pub const SPACE: usize = 32 +  // market
                             4 + 5 * 8 + // vote_tallies (5 outcomes max)
                             4 + 5 * 8 + // stake_weights (5 outcomes max)
                             8 +  // vote_count
                             1 +  // resolution_proposed
                             1 + 1 + // proposed_outcome (Option<u8>)
                             8 +  // proposal_time
                             1 +  // challenge_count
                             1 +  // finalized
                             1 +  // bump
                             40;  // padding
}

#[account]
pub struct VoteAuthority {
    pub market: Pubkey,
    pub authority: Pubkey,
    pub weight: u8,         // Authority weight for multi-sig (1-5)
    pub has_voted: bool,    // Whether this authority has voted
    pub vote: Option<u8>,   // The outcome this authority voted for
    pub bump: u8,
}

impl VoteAuthority {
    pub const SPACE: usize = 32 + // market
                             32 + // authority
                             1 +  // weight
                             1 +  // has_voted
                             1 + 1 + // vote (Option<u8>)
                             1 +  // bump
                             20;  // padding
}

#[derive(Clone, Copy, PartialEq)]
pub enum MarketType {
    TimeBound = 0,
    OpenEnded = 1,
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
    
    #[msg("AI confidence score too low for resolution.")]
    LowAIConfidence,
    
    #[msg("AI resolver is inactive.")]
    ResolverInactive,
    
    #[msg("Market has not yet reached its deadline.")]
    MarketNotExpired,
    
    #[msg("AI resolver can only resolve time-bound markets.")]
    NotTimeBoundMarket,
    
    #[msg("Invalid weight for vote authority. Must be between 1 and 5.")]
    InvalidWeight,
    
    #[msg("Insufficient stake for voting.")]
    InsufficientStake,
    
    #[msg("Voting has not ended yet.")]
    VotingNotEnded,
    
    #[msg("No proposed resolution exists.")]
    NoProposedResolution,
    
    #[msg("Challenge period is still active.")]
    ChallengePeriodActive,
    
    #[msg("Challenge period has ended.")]
    ChallengePeriodEnded,
    
    #[msg("Resolution has already been finalized.")]
    ResolutionFinalized,
}