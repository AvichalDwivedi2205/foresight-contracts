use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

// Emitted events for off-chain indexing
#[event]
pub struct MarketCreatedEvent {
    pub market: Pubkey,
    pub creator: Pubkey,
    pub question: String,
    pub deadline: i64,
    pub market_type: u8,
}

#[event]
pub struct PredictionStakedEvent {
    pub user: Pubkey,
    pub market: Pubkey,
    pub outcome_index: u8,
    pub amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct RewardClaimedEvent {
    pub user: Pubkey,
    pub market: Pubkey,
    pub amount: u64,
    pub outcome_index: u8,
    pub winning_stake: u64,
    pub total_stake: u64,
}

#[event]
pub struct CreatorTierChangedEvent {
    pub creator: Pubkey,
    pub previous_tier: u8,
    pub new_tier: u8,
    pub markets_count: u32,
    pub total_volume: u64,
    pub traction_score: u64,
}

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

        // Emit market creation event for indexer
        emit!(MarketCreatedEvent {
            market: market.key(),
            creator: ctx.accounts.creator.key(),
            question: question.clone(),
            deadline: ai_recommended_resolution_time,
            market_type: ai_classification,
        });

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
        
        let prediction = &mut ctx.accounts.prediction;
        prediction.market = market.key();
        prediction.user = ctx.accounts.user.key();
        prediction.outcome_index = outcome_index;
        prediction.amount = amount;
        prediction.timestamp = current_time;
        prediction.claimed = false;
        prediction.bump = ctx.bumps.prediction;
        
        market.total_pool = market.total_pool.checked_add(amount).unwrap();
        
        if market.stakes_per_outcome.len() < market.outcomes.len() {
            market.stakes_per_outcome = vec![0; market.outcomes.len()];
        }
        
        market.stakes_per_outcome[outcome_index as usize] = 
            market.stakes_per_outcome[outcome_index as usize].checked_add(amount).unwrap();
        
        // Update creator profile stats
        let creator_profile = &mut ctx.accounts.creator_profile;
        creator_profile.total_volume = creator_profile.total_volume.checked_add(amount).unwrap();
        
        // For traction score, we increase it based on activity
        // The score increases more when:
        // - Multiple users participate in a market
        // - Higher amounts are staked
        creator_profile.traction_score = creator_profile.traction_score.checked_add(amount / 1000 + 1).unwrap();
        
        // Update user profile if provided
        if let Some(user_profile) = &mut ctx.accounts.user_profile {
            user_profile.total_staked = user_profile.total_staked.checked_add(amount).unwrap();
            user_profile.total_predictions = user_profile.total_predictions.checked_add(1).unwrap();
            user_profile.last_active_ts = current_time;
        }
        
        // Check if creator tier needs to be updated
        let previous_tier = creator_profile.tier;
        if let Some(creator_tier_threshold) = get_next_tier_threshold(creator_profile) {
            if creator_profile.total_volume >= creator_tier_threshold.0 && 
               creator_profile.markets_created >= creator_tier_threshold.1 && 
               creator_profile.traction_score >= creator_tier_threshold.2 {
                creator_profile.tier = creator_profile.tier.checked_add(1).unwrap();
                
                // Emit event if tier changed
                if creator_profile.tier != previous_tier {
                    emit!(CreatorTierChangedEvent {
                        creator: creator_profile.creator,
                        previous_tier,
                        new_tier: creator_profile.tier,
                        markets_count: creator_profile.markets_created,
                        total_volume: creator_profile.total_volume,
                        traction_score: creator_profile.traction_score,
                    });
                }
            }
        }
        
        // Emit prediction staked event for indexers
        emit!(PredictionStakedEvent {
            user: ctx.accounts.user.key(),
            market: market.key(),
            outcome_index,
            amount,
            timestamp: current_time,
        });
        
        msg!("Staked {} on outcome {}", amount, outcome_index);
        Ok(())
    }

    pub fn vote_market_outcome(
        ctx: Context<VoteMarketOutcome>,
        outcome_index: u8,
    ) -> Result<()> {
        let market = &ctx.accounts.market;
        
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
        
        let admin_key = ctx.accounts.admin.key();
        require!(
            admin_key == ctx.accounts.admin.key(),
            ErrorCode::Unauthorized
        );
        
        require!(!market.resolved, ErrorCode::MarketAlreadyResolved);
        
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
        
        require!(market.resolved, ErrorCode::MarketNotResolved);
        require!(!prediction.claimed, ErrorCode::RewardAlreadyClaimed);

        let winning_outcome = market.winning_outcome.ok_or(ErrorCode::NoWinningOutcome)?;
        require!(
            prediction.outcome_index == winning_outcome,
            ErrorCode::NotWinningPrediction
        );
        
        let total_winning_stakes = market.stakes_per_outcome[winning_outcome as usize];
        require!(total_winning_stakes > 0, ErrorCode::InvalidDistribution);
        
        let user_stake = prediction.amount;
        let total_pool = market.total_pool;
        
        let user_share_numerator = (user_stake as u128).checked_mul(total_pool as u128).unwrap();
        let user_share = user_share_numerator.checked_div(total_winning_stakes as u128).unwrap();
        
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
        
        let reward_amount = user_share
            .checked_sub(creator_fee_amount)
            .unwrap()
            .checked_sub(protocol_fee_amount)
            .unwrap() as u64;
        
        let market_key = market.key();
        let seeds = &[
            b"market".as_ref(),
            market_key.as_ref(),
            &[market.bump],
        ];
        let signer = &[&seeds[..]];
        
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
        
        prediction.claimed = true;
        
        // Update user profile if provided to track winnings
        if let Some(user_profile) = &mut ctx.accounts.user_profile {
            user_profile.total_winnings = user_profile.total_winnings.checked_add(reward_amount).unwrap();
            user_profile.winning_predictions = user_profile.winning_predictions.checked_add(1).unwrap();
            user_profile.last_active_ts = Clock::get()?.unix_timestamp;
        }
        
        // Update creator profile with additional volume and traction
        if let Some(creator_profile) = &mut ctx.accounts.creator_profile {
            // Successful predictions increase traction score more
            creator_profile.traction_score = creator_profile.traction_score.checked_add(reward_amount / 500 + 5).unwrap();
            
            let previous_tier = creator_profile.tier;
            // Check if creator tier needs to be updated
            if let Some(creator_tier_threshold) = get_next_tier_threshold(creator_profile) {
                if creator_profile.total_volume >= creator_tier_threshold.0 && 
                   creator_profile.markets_created >= creator_tier_threshold.1 && 
                   creator_profile.traction_score >= creator_tier_threshold.2 {
                    creator_profile.tier = creator_profile.tier.checked_add(1).unwrap();
                    
                    // Emit event if tier changed
                    if creator_profile.tier != previous_tier {
                        emit!(CreatorTierChangedEvent {
                            creator: creator_profile.creator,
                            previous_tier,
                            new_tier: creator_profile.tier,
                            markets_count: creator_profile.markets_created,
                            total_volume: creator_profile.total_volume,
                            traction_score: creator_profile.traction_score,
                        });
                    }
                }
            }
        }
        
        // Emit reward claimed event for indexers
        emit!(RewardClaimedEvent {
            user: ctx.accounts.user.key(),
            market: market.key(),
            amount: reward_amount,
            outcome_index: prediction.outcome_index,
            winning_stake: user_stake,
            total_stake: total_pool,
        });
        
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
        let voting_deadline = market.deadline.checked_add(15 * 24 * 60 * 60).unwrap();
        
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
        let challenge_deadline = vote_result.proposal_time.checked_add(48 * 60 * 60).unwrap(); 
        
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
        let challenge_deadline = vote_result.proposal_time.checked_add(48 * 60 * 60).unwrap(); 
        
        require!(
            clock.unix_timestamp <= challenge_deadline,
            ErrorCode::ChallengePeriodEnded
        );
        
        // Register challenge
        vote_result.challenge_count = vote_result.challenge_count.checked_add(1).unwrap();
        
        msg!("Resolution challenged with evidence: {}", evidence);
        Ok(())
    }

    pub fn initialize_protocol_stats(ctx: Context<InitializeProtocolStats>) -> Result<()> {
        let stats = &mut ctx.accounts.protocol_stats;
        
        stats.total_volume = 0;
        stats.total_markets = 0;
        stats.total_users = 0;
        stats.total_stakes = 0;
        stats.resolved_markets = Vec::new();
        stats.last_updated_ts = Clock::get()?.unix_timestamp;
        stats.bump = ctx.bumps.protocol_stats;
        
        msg!("Protocol stats initialized");
        Ok(())
    }

    pub fn initialize_user_profile(ctx: Context<InitializeUserProfile>) -> Result<()> {
        let profile = &mut ctx.accounts.user_profile;
        let user = ctx.accounts.user.key();

        profile.user = user;
        profile.total_staked = 0;
        profile.total_winnings = 0;
        profile.total_predictions = 0;
        profile.winning_predictions = 0;
        profile.last_active_ts = Clock::get()?.unix_timestamp;
        profile.bump = ctx.bumps.user_profile;

        msg!("User profile created for {}", user);
        Ok(())
    }

    pub fn update_creator_tier(ctx: Context<UpdateCreatorTier>) -> Result<()> {
        let creator_profile = &mut ctx.accounts.creator_profile;
        let current_tier = creator_profile.tier;
        
        // Tier promotion logic based on activity and volume
        // Tier 0: Beginners
        // Tier 1: Rising (5+ markets, 1000+ volume)
        // Tier 2: Established (20+ markets, 10,000+ volume)
        // Tier 3: Expert (50+ markets, 50,000+ volume, high traction)
        // Tier 4: Elite (100+ markets, 200,000+ volume, very high traction)
        
        let new_tier = if creator_profile.markets_created >= 100 && creator_profile.total_volume >= 200_000 && creator_profile.traction_score >= 1000 {
            4
        } else if creator_profile.markets_created >= 50 && creator_profile.total_volume >= 50_000 && creator_profile.traction_score >= 500 {
            3
        } else if creator_profile.markets_created >= 20 && creator_profile.total_volume >= 10_000 {
            2
        } else if creator_profile.markets_created >= 5 && creator_profile.total_volume >= 1_000 {
            1
        } else {
            0
        };
        
        if new_tier != current_tier {
            creator_profile.tier = new_tier;
            msg!("Creator tier updated from {} to {}", current_tier, new_tier);
        } else {
            msg!("Creator tier remains at {}", current_tier);
        }
        
        Ok(())
    }

    pub fn update_protocol_stats(ctx: Context<UpdateProtocolStats>) -> Result<()> {
        let stats = &mut ctx.accounts.protocol_stats;
        
        // Update total markets
        if let Some(markets) = &ctx.accounts.markets {
            stats.total_markets = stats.total_markets.checked_add(1).unwrap();
        }
        
        // Update total volume if stake occurred
        if let Some(prediction) = &ctx.accounts.prediction {
            stats.total_volume = stats.total_volume.checked_add(prediction.amount).unwrap();
            stats.total_stakes = stats.total_stakes.checked_add(1).unwrap();
        }
        
        // Update total resolved markets if market resolved
        if let Some(market) = &ctx.accounts.market {
            if market.resolved && !stats.resolved_markets.contains(&market.key()) {
                // Keep only the 20 most recent resolved markets
                if stats.resolved_markets.len() >= 20 {
                    stats.resolved_markets.remove(0);
                }
                stats.resolved_markets.push(market.key());
            }
        }
        
        stats.last_updated_ts = Clock::get()?.unix_timestamp;
        msg!("Protocol stats updated");
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
    
    #[account(
        mut,
        seeds = [b"user_profile", user.key().as_ref()],
        bump
    )]
    pub user_profile: Option<Account<'info, UserProfile>>,
    
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
    pub admin: Signer<'info>,
    
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
    
    #[account(
        mut,
        constraint = creator_token_account.owner == market.creator,
        constraint = creator_token_account.mint == market_vault.mint
    )]
    pub creator_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = protocol_fee_account.mint == market_vault.mint
    )]
    pub protocol_fee_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"user_profile", user.key().as_ref()],
        bump
    )]
    pub user_profile: Option<Account<'info, UserProfile>>,
    
    #[account(
        mut,
        seeds = [b"creator_profile", market.creator.as_ref()],
        bump
    )]
    pub creator_profile: Option<Account<'info, CreatorProfile>>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CloseMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
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
        constraint = market_vault.amount == 0 
    )]
    pub market_vault: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RegisterVoteAuthority<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(mut)]
    pub market: Account<'info, Market>,
    
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
    pub admin: Signer<'info>,
    
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
    pub admin: Signer<'info>, 
    
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

#[derive(Accounts)]
pub struct InitializeUserProfile<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        init,
        payer = user,
        space = 8 + UserProfile::SPACE,
        seeds = [b"user_profile", user.key().as_ref()],
        bump
    )]
    pub user_profile: Account<'info, UserProfile>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateCreatorTier<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(mut)]
    pub creator_profile: Account<'info, CreatorProfile>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeProtocolStats<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        init,
        payer = admin,
        space = 8 + ProtocolStats::SPACE,
        seeds = [b"protocol_stats"],
        bump
    )]
    pub protocol_stats: Account<'info, ProtocolStats>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateProtocolStats<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(mut)]
    pub protocol_stats: Account<'info, ProtocolStats>,
    
    #[account(mut)]
    pub market: Option<Account<'info, Market>>,
    
    #[account(mut)]
    pub prediction: Option<Account<'info, Prediction>>,
    
    #[account(mut)]
    pub markets: Option<Account<'info, Market>>,
    
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Default)]
pub struct Market {
    pub creator: Pubkey,
    pub question: String,
    pub outcomes: Vec<String>,
    pub ai_score: f32,
    pub market_type: u8, 
    pub deadline: i64,
    pub ai_suggested_deadline: i64,
    pub resolved: bool,
    pub winning_outcome: Option<u8>,
    pub total_pool: u64,
    pub creator_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub stakes_per_outcome: Vec<u64>, 
    pub ai_resolvable: bool, 
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
    pub authority: Pubkey,       // Ai service pub key
    pub active: bool,            
    pub resolution_count: u64,   
    pub bump: u8,
}

impl AIResolver {
    pub const SPACE: usize = 32 + // authority
                            1 +  // active
                            8 +  // resolution_count
                            1 +  // bump
                            30;  // padding
}

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
pub struct UserProfile {
    pub user: Pubkey,
    pub total_staked: u64,        // Total amount staked across all markets
    pub total_winnings: u64,      // Total amount won from correct predictions
    pub total_predictions: u32,   // Total number of predictions made
    pub winning_predictions: u32, // Number of winning predictions
    pub last_active_ts: i64,      // Timestamp of last activity
    pub bump: u8,
}

impl UserProfile {
    pub const SPACE: usize = 32 + // user
                             8 +  // total_staked
                             8 +  // total_winnings
                             4 +  // total_predictions
                             4 +  // winning_predictions
                             8 +  // last_active_ts
                             1 +  // bump
                             30;  // padding
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

#[account]
pub struct ProtocolStats {
    pub total_volume: u64,        // Total volume across all markets
    pub total_markets: u32,       // Total number of markets
    pub total_users: u32,         // Total number of unique users
    pub total_stakes: u64,        // Total number of stakes made
    pub resolved_markets: Vec<Pubkey>, // Recently resolved markets
    pub last_updated_ts: i64,     // Last time stats were updated
    pub bump: u8,
}

impl ProtocolStats {
    pub const SPACE: usize = 8 +  // total_volume
                             4 +  // total_markets
                             4 +  // total_users
                             8 +  // total_stakes
                             4 + 20 * 32 + // resolved_markets (up to 20 recent markets)
                             8 +  // last_updated_ts
                             1 +  // bump
                             50;  // padding
}

#[derive(Clone, Copy, PartialEq)]
pub enum MarketType {
    TimeBound = 0,
    OpenEnded = 1,
}

// Helper function for tier management
pub fn get_next_tier_threshold(profile: &CreatorProfile) -> Option<(u64, u32, u64)> {
    match profile.tier {
        0 => Some((1_000, 5, 100)),       // Tier 0 -> Tier 1: 1K volume, 5 markets, 100 traction
        1 => Some((10_000, 20, 300)),     // Tier 1 -> Tier 2: 10K volume, 20 markets, 300 traction
        2 => Some((50_000, 50, 500)),     // Tier 2 -> Tier 3: 50K volume, 50 markets, 500 traction
        3 => Some((200_000, 100, 1000)),  // Tier 3 -> Tier 4: 200K volume, 100 markets, 1000 traction
        _ => None,                        // No higher tier available
    }
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