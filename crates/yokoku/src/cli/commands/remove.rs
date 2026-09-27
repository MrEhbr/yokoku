use anyhow::Result;
use clap::Parser;
use yokoku_domain::{ItemId, MediaKind};

use crate::{
    app::App,
    cli::commands::{ItemArgs, confirm_deletion},
    config::Config,
};

#[derive(Parser)]
pub struct Args {
    #[command(flatten)]
    pub item: ItemArgs,

    /// Also delete the item's files
    #[arg(long)]
    pub delete_files: bool,

    /// Delete the item's files without asking
    #[arg(short, long, requires = "delete_files")]
    pub yes: bool,
}

pub async fn run(config: &Config, args: Args) -> Result<()> {
    let app = App::open(config).await?;

    let item = args.item.resolve(&app.library).await?;
    if args.delete_files {
        confirm_deletion(&app.deleter.files_of_item(item).await?, args.yes)?;
    }
    match item {
        ItemId::Series(id) => app.library.remove_series(id, args.delete_files).await?,
        ItemId::Movie(id) => app.library.remove_movie(id, args.delete_files).await?,
    }
    app.deliver_events().await?;

    success!("Removed {} {}", MediaKind::from(args.item.kind), args.item.source)?;
    Ok(())
}
