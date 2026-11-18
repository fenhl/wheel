#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use {
    std::{
        borrow::Cow,
        path::PathBuf,
    },
    toml::toml,
    wheel::fs,
};

include!(concat!(env!("OUT_DIR"), "/build_output.rs"));

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
        path: PathBuf,
        #[clap(long)]
        name: Option<String>,
    },
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] Git(#[from] gix::Error),
    #[error(transparent)] Toml(#[from] toml::ser::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("could not determine crate name from path, try specifying `--name`")]
    Name,
}

#[wheel::main]
async fn main(args: Args) -> Result<(), Error> {
    match args {
        Args::New { kind, mut path, name } => {
            let name = if let Some(name) = name {
                Cow::Owned(name)
            } else {
                Cow::Borrowed(path.file_name().ok_or(Error::Name)?.to_str().ok_or(Error::Name)?)
            };
            fs::create_dir_all(&path).await?;
            gix::init(&path)?;
            //TODO configure pre-commit hook
            fs::write(path.join(".gitignore"), "/.direnv\n/target\n").await?;
            fs::write(path.join(".envrc"), "use flake\n").await?;
            fs::write(path.join("flake.nix"), "{\n    inputs.flake.url = \"github:fenhl/flake\";\n    outputs = attrs: attrs.flake.lib {};\n}\n").await?;
            fs::write(path.join("Cargo.toml"), &toml::to_string_pretty(&toml! {
                [package]
                name = name
                version = "0.1.0"
                edition = EDITION
                //TODO rust-version
            })?).await?;
            path.push("src");
            fs::create_dir(&path).await?;
            match kind {
                CrateKind::Bin => {
                    path.push("main.rs");
                    fs::write(path, "#[global_allocator]\nstatic GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;\n\n#[wheel::main]\nfn main() {}\n").await?;
                }
                CrateKind::Lib => {
                    path.push("lib.rs");
                    fs::write(path, "").await?;
                }
            }
        }
    }
    Ok(())
}
