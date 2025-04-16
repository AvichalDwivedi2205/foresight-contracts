use anchor_lang::prelude::*;

declare_id!("7Gh4eFGmobz5ngu2U3bgZiQm2Adwm33dQTsUwzRb7wBi");

#[program]
pub mod contracts {
    use super::*;

    pub fn initialize_market(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
