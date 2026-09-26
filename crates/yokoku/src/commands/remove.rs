use anyhow::Result;
use clap::Parser;
use yokoku_library::ItemId;

use crate::{app::App, commands::ItemArgs, config::Config};

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub item: ItemArgs,

    /// Also delete the item's files
    #[arg(long)]
    pub delete_files: bool,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;

    match args.item.resolve(&app.library).await? {
        ItemId::Series(id) => app.library.remove_series(id, args.delete_files).await?,
        ItemId::Movie(id) => app.library.remove_movie(id, args.delete_files).await?,
    }

    println!("Removed {} {}", crate::commands::kind_label(args.item.kind.into()), args.item.source);
    Ok(())
}
