use crate::scan::extensions::asset_kind;
use exiftool_rs::tag::Tag;
use std::path::{Path, PathBuf};

pub const MINIMAL_XMP: &[u8] = br#"<?xpacket begin='' id='W5M0MpCehiHzreSzNTczkc9d'?>
<x:xmpmeta xmlns:x='adobe:ns:meta/'>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
<rdf:Description rdf:about=''/>
</rdf:RDF>
</x:xmpmeta>
<?xpacket end='w'?>"#;

pub fn uses_xmp_sidecar_write(path: &Path) -> bool {
    let ext = crate::path_util::os_extension(path);
    matches!(asset_kind(&ext), "video" | "raw")
}

pub fn xmp_sidecar_path(media_path: &Path) -> PathBuf {
    let file_name = media_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    media_path.with_file_name(format!("{}.xmp", file_name))
}

pub fn workspace_sidecar_path(workspace_xmp_dir: &Path, root_id: i64, rel_path: &str) -> PathBuf {
    workspace_xmp_dir
        .join(root_id.to_string())
        .join(format!("{}.xmp", rel_path))
}

pub fn merge_tags(embedded: Vec<Tag>, sidecar: Vec<Tag>) -> Vec<Tag> {
    let mut merged = embedded;
    for tag in sidecar {
        if let Some(existing) = merged.iter_mut().find(|t| t.name == tag.name) {
            *existing = tag;
        } else {
            merged.push(tag);
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use exiftool_rs::tag::{Tag, TagGroup, TagId};
    use exiftool_rs::value::Value;
    use std::path::Path;

    fn tag(name: &str, value: &str) -> Tag {
        Tag {
            id: TagId::Text(name.into()),
            name: name.into(),
            description: String::new(),
            group: TagGroup {
                family0: "XMP".into(),
                family1: String::new(),
                family2: String::new(),
                family3: exiftool_rs::tag::MAIN_DOCUMENT.into(),
            },
            raw_value: Value::String(value.into()),
            print_value: value.into(),
            priority: 1,
        }
    }

    #[test]
    fn detects_video_and_raw_sidecar_writes() {
        assert!(uses_xmp_sidecar_write(Path::new("/a/clip.mov")));
        assert!(uses_xmp_sidecar_write(Path::new("/a/photo.cr2")));
        assert!(!uses_xmp_sidecar_write(Path::new("/a/photo.jpg")));
    }

    #[test]
    fn builds_sidecar_path_from_media_path() {
        assert_eq!(
            xmp_sidecar_path(Path::new("/photos/sample.mov")),
            Path::new("/photos/sample.mov.xmp")
        );
    }

    #[test]
    fn builds_workspace_sidecar_path() {
        assert_eq!(
            workspace_sidecar_path(Path::new("/ws/xmp"), 3, "photos/a7c.jpg"),
            Path::new("/ws/xmp/3/photos/a7c.jpg.xmp")
        );
    }

    #[test]
    fn merge_tags_overrides_embedded_and_appends_new() {
        let merged = merge_tags(
            vec![tag("Rating", "3"), tag("Keywords", "old")],
            vec![tag("Rating", "5"), tag("Keywords", "travel")],
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].print_value, "5");
        assert_eq!(merged[1].print_value, "travel");
    }
}
