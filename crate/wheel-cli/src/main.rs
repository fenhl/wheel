#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    tokio::process::Command,
    wheel::traits::AsyncCommandOutputExt as _,
};

#[derive(Clone, Copy, clap::ValueEnum)]
enum CrateKind {
    Bin,
    Lib,
}

#[derive(clap::Parser)]
#[clap(version)]
enum Args {
    New {
        kind: CrateKind,
        path: String,
    },
}

#[wheel::main]
async fn main(args: Args) -> wheel::Result {
    match args {
        Args::New { kind, path } => {
            Command::new("cargo").arg("new").arg(match kind {
                CrateKind::Bin => "--bin",
                CrateKind::Lib => "--lib",
            }).arg(path).check("cargo new").await?;
        }
    }
    Ok(())
}
