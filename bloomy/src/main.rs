fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> bloomy::error::Result<()> {
    let config_path = config_path_from_args(std::env::args().skip(1))?
        .map(Ok)
        .unwrap_or_else(resolve_default_config_path)?;

    let config = bloomy::BloomyConfig::load_or_create(&config_path)?;
    let _bloomy = bloomy::Bloomy::open(config.clone().into())?;

    println!(
        "loaded {} with storage_path={} memtable_bytes={}",
        config_path.display(),
        config.storage_path.display(),
        config.memtable_bytes
    );

    Ok(())
}

fn resolve_default_config_path() -> bloomy::error::Result<std::path::PathBuf> {
    let local_config = std::env::current_dir()?.join("bloomy.json");

    if local_config.exists() {
        return Ok(local_config);
    }

    bloomy::default_config_path()
}

fn config_path_from_args(
    mut args: impl Iterator<Item = String>,
) -> bloomy::error::Result<Option<std::path::PathBuf>> {
    let Some(flag) = args.next() else {
        return Ok(None);
    };

    if flag != "--config" {
        return Err(bloomy::error::Error::Message(format!(
            "unknown argument: {flag}"
        )));
    }

    let Some(path) = args.next() else {
        return Err(bloomy::error::Error::Message(format!(
            "{flag} requires a path"
        )));
    };

    if let Some(extra) = args.next() {
        return Err(bloomy::error::Error::Message(format!(
            "unknown argument: {extra}"
        )));
    }

    Ok(Some(path.into()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn config_path_from_args_accepts_config_path() {
        let path = config_path_from_args(
            ["--config".to_string(), "local/bloomy.json".to_string()].into_iter(),
        )
        .unwrap();

        assert_eq!(path, Some(PathBuf::from("local/bloomy.json")));
    }

    #[test]
    fn config_path_from_args_rejects_binary_alias() {
        let error = config_path_from_args(
            ["--binary".to_string(), "local/bloomy.json".to_string()].into_iter(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown argument: --binary"));
    }
}
