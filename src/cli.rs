/// Splits a CLI path list into source paths and a destination path.
///
/// Commands in this crate expect one or more source paths followed by the
/// destination as the final argument.
pub fn split_sources_and_dest(paths: &[String]) -> Option<(&[String], &str)> {
    let (dest, sources) = paths.split_last()?;
    if sources.is_empty() {
        return None;
    }

    Some((sources, dest.as_str()))
}

#[cfg(test)]
mod tests {
    use super::split_sources_and_dest;

    #[test]
    fn requires_at_least_one_source_and_one_destination() {
        assert!(split_sources_and_dest(&[]).is_none());
        assert!(split_sources_and_dest(&["dest".to_owned()]).is_none());
    }

    #[test]
    fn returns_sources_and_destination() {
        let paths = vec![
            "/music/source-a".to_owned(),
            "/music/source-b".to_owned(),
            "/music/dest".to_owned(),
        ];

        let (sources, dest) = split_sources_and_dest(&paths).expect("expected valid path list");

        assert_eq!(sources, &paths[..2]);
        assert_eq!(dest, "/music/dest");
    }
}
