#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct ConvertUtilityConfig {
    #[arg(short, long)]
    input: String,
}

fn main() -> anyhow::Result<()> {
    Ok(())
}
