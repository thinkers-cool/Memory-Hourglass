use crate::dates::CAPTURE_AT_SQL;
use crate::sort::SortField;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating_min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_states: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_to: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_ids: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_ids: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_gps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_duplicate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_ids: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_only: Option<bool>,
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub(crate) fn apply_sort(
    builder: &mut QueryBuilder<'_, sqlx::Sqlite>,
    field: SortField,
    desc: bool,
) {
    let dir = if desc { "DESC" } else { "ASC" };
    let tie = if desc { " DESC" } else { " ASC" };

    builder.push(" ORDER BY ");
    match field {
        SortField::Name => {
            builder.push("a.file_name ").push(dir);
        }
        SortField::Rating => {
            builder.push("m.rating ").push(dir).push(", a.id").push(tie);
        }
        SortField::Path => {
            builder.push("a.rel_path ").push(dir);
        }
        SortField::Date => {
            builder
                .push(CAPTURE_AT_SQL)
                .push(" ")
                .push(dir)
                .push(", a.id")
                .push(tie);
        }
    }
}

fn apply_sync_states_filter(builder: &mut QueryBuilder<'_, sqlx::Sqlite>, sync_states: &[String]) {
    builder.push(" AND a.sync_state IN (");
    for (index, state) in sync_states.iter().enumerate() {
        if index > 0 {
            builder.push(", ");
        }
        builder.push_bind(state.clone());
    }
    builder.push(")");
}

fn apply_album_ids_filter(builder: &mut QueryBuilder<'_, sqlx::Sqlite>, album_ids: &[i64]) {
    builder.push(
        " AND EXISTS (SELECT 1 FROM album_item ai WHERE ai.asset_id = a.id AND ai.album_id IN (",
    );
    for (index, album_id) in album_ids.iter().enumerate() {
        if index > 0 {
            builder.push(", ");
        }
        builder.push_bind(*album_id);
    }
    builder.push("))");
}

pub(crate) fn apply_deleted_clause(
    builder: &mut QueryBuilder<'_, sqlx::Sqlite>,
    filter: &AssetFilter,
) {
    if filter.deleted_only == Some(true) {
        builder.push("a.deleted_at IS NOT NULL");
    } else {
        builder.push("a.deleted_at IS NULL");
    }
}

pub(crate) fn apply_filter(builder: &mut QueryBuilder<'_, sqlx::Sqlite>, filter: &AssetFilter) {
    if let Some(root_id) = filter.root_id {
        builder.push(" AND a.root_id = ").push_bind(root_id);
    }
    if let Some(rating_min) = filter.rating_min {
        builder.push(" AND m.rating >= ").push_bind(rating_min);
    }
    if let Some(sync_states) = filter.sync_states.clone() {
        if !sync_states.is_empty() {
            apply_sync_states_filter(builder, &sync_states);
        }
    }
    if let Some(kind) = filter.kind.clone() {
        builder.push(" AND a.kind = ").push_bind(kind);
    }
    if let Some(camera) = filter.camera.clone() {
        let pattern = format!("%{}%", escape_like(&camera));
        builder
            .push(" AND m.camera LIKE ")
            .push_bind(pattern)
            .push(" ESCAPE '\\'");
    }
    if filter.has_gps == Some(true) {
        builder.push(" AND m.latitude IS NOT NULL AND m.longitude IS NOT NULL");
    }
    if filter.has_duplicate == Some(true) {
        builder.push(" AND a.has_duplicate = 1");
    }
    if let Some(from) = filter.capture_from {
        builder
            .push(" AND ")
            .push(CAPTURE_AT_SQL)
            .push(" >= ")
            .push_bind(from);
    }
    if let Some(to) = filter.capture_to {
        builder
            .push(" AND ")
            .push(CAPTURE_AT_SQL)
            .push(" <= ")
            .push_bind(to);
    }
    if let Some(tag_ids) = filter.tag_ids.clone() {
        if !tag_ids.is_empty() {
            builder.push(" AND (");
            for (i, tag_id) in tag_ids.iter().enumerate() {
                if i > 0 {
                    builder.push(" OR ");
                }
                builder
                    .push("EXISTS (SELECT 1 FROM asset_tag at WHERE at.asset_id = a.id AND at.tag_id = ")
                    .push_bind(*tag_id)
                    .push(")");
            }
            builder.push(")");
        }
    }
    if let Some(album_ids) = filter.album_ids.clone() {
        if !album_ids.is_empty() {
            apply_album_ids_filter(builder, &album_ids);
        }
    }
    if let Some(query) = filter.meta_search.as_ref() {
        let trimmed = query.trim();
        if !trimmed.is_empty() {
            let pattern = format!("%{}%", escape_like(trimmed));
            builder
                .push(" AND (EXISTS (SELECT 1 FROM asset_raw_tag rt WHERE rt.asset_id = a.id AND (rt.name LIKE ")
                .push_bind(pattern.clone())
                .push(" ESCAPE '\\' OR rt.value LIKE ")
                .push_bind(pattern.clone())
                .push(" ESCAPE '\\')) OR a.file_name LIKE ")
                .push_bind(pattern.clone())
                .push(" ESCAPE '\\' OR m.camera LIKE ")
                .push_bind(pattern.clone())
                .push(" ESCAPE '\\' OR m.lens LIKE ")
                .push_bind(pattern)
                .push(" ESCAPE '\\')");
        }
    }
    if let Some(ids) = filter.asset_ids.clone() {
        if ids.is_empty() {
            builder.push(" AND 1=0");
        } else {
            builder.push(" AND a.id IN (");
            for (i, id) in ids.iter().enumerate() {
                if i > 0 {
                    builder.push(", ");
                }
                builder.push_bind(*id);
            }
            builder.push(")");
        }
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct Row {
    pub(crate) id: i64,
    pub(crate) file_name: String,
    pub(crate) ext: String,
    pub(crate) kind: String,
    pub(crate) capture_at: Option<i64>,
    pub(crate) rating: Option<i64>,
    pub(crate) sync_state: String,
    pub(crate) thumb_key: Option<String>,
    pub(crate) has_duplicate: i64,
    pub(crate) root_path: String,
    pub(crate) rel_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_like_escapes_wildcards() {
        assert_eq!(escape_like("50%_x"), "50\\%\\_x");
    }

    #[test]
    fn escape_like_escapes_backslash() {
        assert_eq!(escape_like(r"a\b"), r"a\\b");
    }
}
