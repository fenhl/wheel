use {
    std::{
        env,
        fs::File,
        io::{
            self,
            prelude::*,
        },
        path::Path,
    },
    cargo_metadata::MetadataCommand,
};

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)] CargoMetadata(#[from] cargo_metadata::Error),
    #[error(transparent)] Io(#[from] io::Error),
}

fn main() -> Result<(), Error> {
    let metadata = MetadataCommand::default().exec()?;
    let pkgid = metadata.workspace_members.first().unwrap();
    let edition = metadata.packages.into_iter().find(|pkg| pkg.id == *pkgid).unwrap().edition;
    let mut out_f = File::create(Path::new(&env::var_os("OUT_DIR").unwrap()).join("build_output.rs"))?;
    writeln!(&mut out_f, "const EDITION: &str = {:?};", edition.as_str())?;
    Ok(())
}
