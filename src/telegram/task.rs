use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(alias = "te")]
pub struct LupinTelegramGet {}

impl LupinTelegramGet {
    pub async fn execute(&self) -> eyre::Result<()> {
        Ok(())
    }
}
