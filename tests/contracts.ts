import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Contracts } from "../target/types/contracts";

// Program ID from declare_id!
export const PROGRAM_ID = new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

// Market types enum matching the Rust definition
export enum MarketType {
  TimeBound = 0,
  OpenEnded = 1,
}

// Program class
export class PredictionMarketClient {
  readonly program: Program<Contracts>;
  readonly provider: anchor.AnchorProvider;

  constructor(
    provider: anchor.AnchorProvider,
    programId: PublicKey = PROGRAM_ID
  ) {
    this.provider = provider;
    this.program = new anchor.Program(
      require("../target/idl/contracts.json"),
      provider
    ) as Program<Contracts>;    
  }

  // Helper functions for finding PDAs
  async findCreatorProfileAddress(creator: PublicKey): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("creator_profile"), creator.toBuffer()],
      this.program.programId
    );
  }

  async findUserProfileAddress(user: PublicKey): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("user_profile"), user.toBuffer()],
      this.program.programId
    );
  }

  async findAIResolverAddress(authority: PublicKey): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("ai_resolver"), authority.toBuffer()],
      this.program.programId
    );
  }

  async findProtocolStatsAddress(): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("protocol_stats")],
      this.program.programId
    );
  }

  async findMarketAddress(
    creator: PublicKey,
    marketIndex: number
  ): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("market"), creator.toBuffer(), new Uint8Array(Buffer.from(new BN(marketIndex).toArray("le", 4)))],
      this.program.programId
    );
  }

  async findMarketVaultAddress(market: PublicKey): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("market_vault"), market.toBuffer()],
      this.program.programId
    );
  }

  async findPredictionAddress(
    market: PublicKey,
    user: PublicKey
  ): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("prediction"), market.toBuffer(), user.toBuffer()],
      this.program.programId
    );
  }

  async findVoteResultAddress(market: PublicKey): Promise<[PublicKey, number]> {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("vote_result"), market.toBuffer()],
      this.program.programId
    );
  }

  // Instruction methods

  async createCreatorProfile(creator: anchor.web3.Keypair): Promise<string> {
    return this.program.methods
      .createCreatorProfile()
      .accounts({
        creator: creator.publicKey,
      })
      .signers([creator])
      .rpc();
  }

  async initializeAiResolver(admin: anchor.web3.Keypair): Promise<string> {
    return this.program.methods
      .initializeAiResolver()
      .accounts({
        admin: admin.publicKey,
      })
      .signers([admin])
      .rpc();
  }

  async resolveMarketViaAi(
    resolverAuthority: anchor.web3.Keypair,
    market: PublicKey,
    winningOutcomeIndex: number,
    aiConfidenceScore: number,
    resolutionData: string
  ): Promise<string> {
    return this.program.methods
      .resolveMarketViaAi(
        winningOutcomeIndex,
        aiConfidenceScore,
        resolutionData
      )
      .accounts({
        resolverAuthority: resolverAuthority.publicKey,
        market,
      })
      .signers([resolverAuthority])
      .rpc();
  }

  async createMarket(
    creator: anchor.web3.Keypair,
    mint: PublicKey,
    question: string,
    outcomes: string[],
    aiScore: number,
    aiRecommendedResolutionTime: BN,
    aiClassification: number,
    creatorMetadata: string,
    creatorFeeBps?: number,
    aiResolvable?: boolean
  ): Promise<string> {
    return this.program.methods
      .createMarket(
        question,
        outcomes,
        aiScore,
        aiRecommendedResolutionTime,
        aiClassification,
        creatorMetadata,
        creatorFeeBps ? creatorFeeBps : null,
        aiResolvable !== undefined ? aiResolvable : null
      )
      .accounts({
        creator: creator.publicKey,
        mint,
      })
      .signers([creator])
      .rpc();
  }

  async stakePrediction(
    user: anchor.web3.Keypair,
    market: PublicKey,
    userTokenAccount: PublicKey,
    outcomeIndex: number,
    amount: BN
  ): Promise<string> {
    return this.program.methods
      .stakePrediction(
        outcomeIndex,
        amount
      )
      .accounts({
        user: user.publicKey,
        market,
        userTokenAccount,
      })
      .signers([user])
      .rpc();
  }

  async voteMarketOutcome(
    voter: anchor.web3.Keypair,
    market: PublicKey,
    outcomeIndex: number
  ): Promise<string> {
    return this.program.methods
      .voteMarketOutcome(
        outcomeIndex
      )
      .accounts({
        voter: voter.publicKey,
        market,
      })
      .signers([voter])
      .rpc();
  }

  async resolveMarket(
    admin: anchor.web3.Keypair,
    market: PublicKey,
    winningOutcomeIndex?: number
  ): Promise<string> {
    return this.program.methods
      .resolveMarket(
        winningOutcomeIndex !== undefined ? winningOutcomeIndex : null
      )
      .accounts({
        admin: admin.publicKey,
        market,
      })
      .signers([admin])
      .rpc();
  }

  async claimReward(
    user: anchor.web3.Keypair,
    market: PublicKey,
    userTokenAccount: PublicKey,
    creatorTokenAccount: PublicKey,
    protocolFeeAccount: PublicKey
  ): Promise<string> {
    return this.program.methods
      .claimReward()
      .accounts({
        user: user.publicKey,
        market,
        userTokenAccount,
        creatorTokenAccount,
        protocolFeeAccount,
      })
      .signers([user])
      .rpc();
  }

  async closeMarket(
    admin: anchor.web3.Keypair,
    market: PublicKey,
    marketVault: PublicKey
  ): Promise<string> {
    return this.program.methods
      .closeMarket()
      .accountsPartial({
        admin: admin.publicKey,
        market,
        marketVault,
      })      
      .signers([admin])
      .rpc();
  }

  async registerVoteAuthority(
    admin: anchor.web3.Keypair,
    market: PublicKey,
    authority: PublicKey,
    weight: number
  ): Promise<string> {
    return this.program.methods
      .registerVoteAuthority(
        weight
      )
      .accounts({
        admin: admin.publicKey,
        market,
        authority,
      })
      .signers([admin])
      .rpc();
  }

  async initializeVoteResult(
    admin: anchor.web3.Keypair,
    market: PublicKey
  ): Promise<string> {
    return this.program.methods
      .initializeVoteResult()
      .accounts({
        admin: admin.publicKey,
        market,
      })
      .signers([admin])
      .rpc();
  }

  async stakeWeightedVote(
    voter: anchor.web3.Keypair,
    market: PublicKey,
    outcomeIndex: number
  ): Promise<string> {
    return this.program.methods
      .stakeWeightedVote(
        outcomeIndex
      )
      .accounts({
        voter: voter.publicKey,
        market,
      })
      .signers([voter])
      .rpc();
  }

  async proposeResolution(
    authority: anchor.web3.Keypair,
    market: PublicKey,
    outcomeIndex: number
  ): Promise<string> {
    return this.program.methods
      .proposeResolution(
        outcomeIndex
      )
      .accounts({
        authority: authority.publicKey,
        market,
      })
      .signers([authority])
      .rpc();
  }

  async finalizeResolution(
    admin: anchor.web3.Keypair,
    market: PublicKey
  ): Promise<string> {
    return this.program.methods
      .finalizeResolution()
      .accounts({
        admin: admin.publicKey,
        market,
      })
      .signers([admin])
      .rpc();
  }

  async challengeResolution(
    challenger: anchor.web3.Keypair,
    market: PublicKey,
    evidence: string
  ): Promise<string> {
    return this.program.methods
      .challengeResolution(
        evidence
      )
      .accounts({
        challenger: challenger.publicKey,
        market,
      })
      .signers([challenger])
      .rpc();
  }

  async initializeProtocolStats(
    admin: anchor.web3.Keypair
  ): Promise<string> {
    return this.program.methods
      .initializeProtocolStats()
      .accounts({
        admin: admin.publicKey,
      })
      .signers([admin])
      .rpc();
  }

  async initializeUserProfile(
    user: anchor.web3.Keypair
  ): Promise<string> {
    return this.program.methods
      .initializeUserProfile()
      .accounts({
        user: user.publicKey,
      })
      .signers([user])
      .rpc();
  }

  async updateCreatorTier(
    admin: anchor.web3.Keypair,
    creatorProfile: PublicKey
  ): Promise<string> {
    return this.program.methods
      .updateCreatorTier()
      .accountsStrict({
        admin: admin.publicKey,
        creatorProfile,
        systemProgram: SystemProgram.programId,
      })
      .signers([admin])
      .rpc();
  }

  async updateProtocolStats(
    admin: anchor.web3.Keypair,
    market?: PublicKey,
    prediction?: PublicKey,
    markets?: PublicKey
  ): Promise<string> {
    const accounts: any = {
      admin: admin.publicKey,
    };
    
    if (market) accounts.market = market;
    if (prediction) accounts.prediction = prediction;
    if (markets) accounts.markets = markets;

    return this.program.methods
      .updateProtocolStats()
      .accountsPartial(accounts)
      .signers([admin])
      .rpc();
  }
}
