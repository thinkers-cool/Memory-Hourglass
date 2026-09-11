use crate::catalog::models::LinkedAsset;

pub fn resolve_display_path(abs_path: &str, kind: &str, links: &[LinkedAsset]) -> String {
    if matches!(kind, "raw" | "video") {
        if let Some(link) = links.iter().find(|entry| entry.asset_kind == "image") {
            return format!("{}/{}", link.root_path, link.rel_path);
        }
    }
    abs_path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::models::LinkedAsset;

    #[test]
    fn resolves_raw_to_linked_jpeg() {
        let links = vec![LinkedAsset {
            kind: "jpeg".into(),
            id: 2,
            file_name: "DSC.jpg".into(),
            asset_kind: "image".into(),
            root_path: "/photos".into(),
            rel_path: "DSC.jpg".into(),
        }];
        let resolved = resolve_display_path("/photos/DSC.arw", "raw", &links);
        assert_eq!(resolved, "/photos/DSC.jpg");
    }

    #[test]
    fn keeps_abs_path_when_no_link() {
        let resolved = resolve_display_path("/photos/DSC.arw", "raw", &[]);
        assert_eq!(resolved, "/photos/DSC.arw");
    }
}
