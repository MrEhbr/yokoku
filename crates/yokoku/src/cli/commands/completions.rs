use std::io;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;

#[derive(Parser)]
pub struct Args {
    pub shell: Shell,
}

pub fn run(args: &Args) {
    clap_complete::generate(args.shell, &mut crate::cli::args::Args::command(), "yokoku", &mut io::stdout());
}
