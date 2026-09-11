pub fn is_index_complete(
    mtime_ns: i64,
    indexed_mtime_ns: Option<i64>,
    thumb_key: Option<&str>,
    kind: &str,
) -> bool {
    if indexed_mtime_ns != Some(mtime_ns) {
        return false;
    }
    if matches!(kind, "image" | "video" | "raw") && thumb_key.is_none() {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_when_marker_and_thumb_match() {
        assert!(is_index_complete(100, Some(100), Some("1.webp"), "image"));
    }

    #[test]
    fn incomplete_when_index_marker_missing() {
        assert!(!is_index_complete(100, None, Some("1.webp"), "image"));
    }

    #[test]
    fn incomplete_when_index_marker_stale() {
        assert!(!is_index_complete(200, Some(100), Some("1.webp"), "image"));
    }

    #[test]
    fn incomplete_when_thumb_missing_for_media() {
        assert!(!is_index_complete(100, Some(100), None, "video"));
    }

    #[test]
    fn complete_without_thumb_for_non_media() {
        assert!(is_index_complete(100, Some(100), None, "other"));
    }
}
