use crate::activity::models::actor;
use crate::activity::models::event_type;
use crate::activity::models::ActivityInput;
use crate::activity::recorder::ActivityRecorder;
use crate::catalog::models::AssetMetaPatch;
use crate::error::Result;
use crate::scan::ScanSummary;
use crate::trace::asset_subject_key;
use serde_json::json;

pub async fn record_soft_deleted(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    asset_ids: &[i64],
    deleted_at: i64,
) -> Result<i64> {
    let payload = json!({
        "asset_ids": asset_ids,
        "deleted_at": deleted_at,
    });
    let revert = json!({ "asset_ids": asset_ids });
    recorder
        .append(ActivityInput {
            event_type: event_type::ASSET_SOFT_DELETED.to_string(),
            actor: actor::USER.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("asset".into()),
            subject_id: asset_ids.first().copied(),
            subject_key: None,
            summary: Some(format!("soft deleted {} assets", asset_ids.len())),
            payload_json: payload.to_string(),
            revert_json: Some(revert.to_string()),
        })
        .await
}

pub async fn record_restored(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    deleted_at_by_id: &std::collections::HashMap<i64, i64>,
) -> Result<i64> {
    let asset_ids: Vec<i64> = deleted_at_by_id.keys().copied().collect();
    let items = deleted_at_by_id
        .iter()
        .map(|(asset_id, deleted_at)| {
            json!({
                "asset_id": asset_id,
                "deleted_at": deleted_at,
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({ "asset_ids": asset_ids });
    let revert = json!({ "items": items });
    recorder
        .append(ActivityInput {
            event_type: event_type::ASSET_RESTORED.to_string(),
            actor: actor::USER.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("asset".into()),
            subject_id: asset_ids.first().copied(),
            subject_key: None,
            summary: Some(format!("restored {} assets", asset_ids.len())),
            payload_json: payload.to_string(),
            revert_json: Some(revert.to_string()),
        })
        .await
}

pub async fn record_purged(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    asset_ids: &[i64],
) -> Result<i64> {
    let payload = json!({ "asset_ids": asset_ids });
    recorder
        .append(ActivityInput {
            event_type: event_type::ASSET_PURGED.to_string(),
            actor: actor::USER.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("asset".into()),
            subject_id: asset_ids.first().copied(),
            subject_key: None,
            summary: Some(format!("purged {} assets", asset_ids.len())),
            payload_json: payload.to_string(),
            revert_json: None,
        })
        .await
}

pub async fn record_metadata_changed(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    asset_id: i64,
    root_id: i64,
    rel_path: &str,
    before: &AssetMetaPatch,
    after: &AssetMetaPatch,
) -> Result<i64> {
    let subject_key = asset_subject_key(root_id, rel_path);
    let payload = json!({
        "asset_id": asset_id,
        "root_id": root_id,
        "rel_path": rel_path,
        "before": before,
        "after": after,
    });
    let revert = json!({
        "items": [{ "asset_id": asset_id, "patch": before }],
    });
    recorder
        .append(ActivityInput {
            event_type: event_type::ASSET_METADATA_CHANGED.to_string(),
            actor: actor::USER.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("asset".into()),
            subject_id: Some(asset_id),
            subject_key: Some(subject_key),
            summary: None,
            payload_json: payload.to_string(),
            revert_json: Some(revert.to_string()),
        })
        .await
}

pub async fn record_batch_metadata_changed(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    items: &[(i64, i64, String, AssetMetaPatch, AssetMetaPatch)],
) -> Result<i64> {
    if items.is_empty() {
        return Ok(0);
    }
    let revert_items = items
        .iter()
        .map(|(asset_id, _, _, before, _)| json!({ "asset_id": asset_id, "patch": before }))
        .collect::<Vec<_>>();
    let payload = json!({
        "count": items.len(),
        "items": items.iter().map(|(asset_id, root_id, rel_path, before, after)| {
            json!({
                "asset_id": asset_id,
                "root_id": root_id,
                "rel_path": rel_path,
                "before": before,
                "after": after,
            })
        }).collect::<Vec<_>>(),
    });
    let (first_id, root_id, rel_path, _, _) = &items[0];
    recorder
        .append(ActivityInput {
            event_type: event_type::ASSET_METADATA_CHANGED.to_string(),
            actor: actor::USER.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("asset".into()),
            subject_id: Some(*first_id),
            subject_key: Some(asset_subject_key(*root_id, rel_path)),
            summary: Some(format!("metadata changed on {} assets", items.len())),
            payload_json: payload.to_string(),
            revert_json: Some(json!({ "items": revert_items }).to_string()),
        })
        .await
}

pub async fn record_tags_added(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    asset_ids: &[i64],
    tag_id: i64,
) -> Result<i64> {
    let payload = json!({ "asset_ids": asset_ids, "tag_id": tag_id });
    let revert = json!({ "asset_ids": asset_ids, "tag_id": tag_id });
    recorder
        .append(ActivityInput {
            event_type: event_type::ASSET_TAGS_ADDED.to_string(),
            actor: actor::USER.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("asset".into()),
            subject_id: asset_ids.first().copied(),
            subject_key: None,
            summary: Some(format!(
                "tag {} added to {} assets",
                tag_id,
                asset_ids.len()
            )),
            payload_json: payload.to_string(),
            revert_json: Some(revert.to_string()),
        })
        .await
}

pub async fn record_tags_removed(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    asset_ids: &[i64],
    tag_id: i64,
) -> Result<i64> {
    let payload = json!({ "asset_ids": asset_ids, "tag_id": tag_id });
    let revert = json!({ "asset_ids": asset_ids, "tag_id": tag_id });
    recorder
        .append(ActivityInput {
            event_type: event_type::ASSET_TAGS_REMOVED.to_string(),
            actor: actor::USER.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("asset".into()),
            subject_id: asset_ids.first().copied(),
            subject_key: None,
            summary: Some(format!(
                "tag {} removed from {} assets",
                tag_id,
                asset_ids.len()
            )),
            payload_json: payload.to_string(),
            revert_json: Some(revert.to_string()),
        })
        .await
}

pub async fn record_scan_completed(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    root_id: i64,
    summary: &ScanSummary,
) -> Result<i64> {
    let payload = json!({
        "root_id": root_id,
        "new_count": summary.new_count,
        "modified_count": summary.modified_count,
        "missing_count": summary.missing_count,
        "scanned": summary.scanned,
        "indexed": summary.indexed,
    });
    recorder
        .append(ActivityInput {
            event_type: event_type::SCAN_COMPLETED.to_string(),
            actor: actor::SCAN.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("root".into()),
            subject_id: Some(root_id),
            subject_key: None,
            summary: Some(format!("scan completed for root {}", root_id)),
            payload_json: payload.to_string(),
            revert_json: None,
        })
        .await
}

pub async fn record_export_completed(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    job_id: i64,
    asset_count: usize,
    destination: &str,
) -> Result<i64> {
    let payload = json!({
        "job_id": job_id,
        "asset_count": asset_count,
        "destination": destination,
    });
    recorder
        .append(ActivityInput {
            event_type: event_type::EXPORT_COMPLETED.to_string(),
            actor: actor::EXPORT.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("export".into()),
            subject_id: Some(job_id),
            subject_key: None,
            summary: Some(format!("export completed: {} assets", asset_count)),
            payload_json: payload.to_string(),
            revert_json: None,
        })
        .await
}

pub async fn record_catalog_rebuilt(
    recorder: &ActivityRecorder,
    correlation_id: Option<&str>,
    roots_count: usize,
) -> Result<i64> {
    let payload = json!({ "roots_count": roots_count });
    recorder
        .append(ActivityInput {
            event_type: event_type::CATALOG_REBUILT.to_string(),
            actor: actor::SYSTEM.to_string(),
            correlation_id: correlation_id.map(str::to_string),
            subject_type: Some("workspace".into()),
            subject_id: None,
            subject_key: None,
            summary: Some("catalog rebuilt".into()),
            payload_json: payload.to_string(),
            revert_json: None,
        })
        .await
}

pub async fn record_asset_missing(
    recorder: &ActivityRecorder,
    root_id: i64,
    rel_paths: &[String],
) -> Result<()> {
    if rel_paths.is_empty() {
        return Ok(());
    }
    for rel_path in rel_paths {
        let subject_key = asset_subject_key(root_id, rel_path);
        recorder
            .append(ActivityInput {
                event_type: event_type::ASSET_MISSING.to_string(),
                actor: actor::SCAN.to_string(),
                correlation_id: None,
                subject_type: Some("asset".into()),
                subject_id: None,
                subject_key: Some(subject_key),
                summary: Some(format!("file missing: {}", rel_path)),
                payload_json: json!({ "root_id": root_id, "rel_path": rel_path }).to_string(),
                revert_json: None,
            })
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activity::recorder::ActivityRecorder;
    use crate::catalog::models::AssetMetaPatch;
    use crate::catalog::Catalog;
    use tempfile::tempdir;

    #[tokio::test]
    async fn record_helpers_append_activity_rows() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let recorder = ActivityRecorder::new(catalog.pools().clone());

        assert_eq!(
            record_batch_metadata_changed(&recorder, None, &[])
                .await
                .unwrap(),
            0
        );
        assert_eq!(record_asset_missing(&recorder, 1, &[]).await.unwrap(), ());

        let soft_id = record_soft_deleted(&recorder, Some("c"), &[1, 2], 10)
            .await
            .unwrap();
        let restore_id = record_restored(
            &recorder,
            Some("c"),
            &std::collections::HashMap::from([(1, 10)]),
        )
        .await
        .unwrap();
        let purge_id = record_purged(&recorder, Some("c"), &[1]).await.unwrap();
        let scan_id = record_scan_completed(
            &recorder,
            Some("c"),
            1,
            &ScanSummary {
                scanned: 2,
                indexed: 2,
                new_count: 1,
                modified_count: 0,
                missing_count: 0,
            },
        )
        .await
        .unwrap();
        let export_id = record_export_completed(&recorder, Some("c"), 9, 3, "/tmp/out")
            .await
            .unwrap();
        let rebuild_id = record_catalog_rebuilt(&recorder, Some("c"), 2)
            .await
            .unwrap();
        let tag_add = record_tags_added(&recorder, Some("c"), &[1], 5)
            .await
            .unwrap();
        let tag_remove = record_tags_removed(&recorder, Some("c"), &[1], 5)
            .await
            .unwrap();
        record_asset_missing(
            &recorder,
            1,
            &[String::from("missing-a.jpg"), String::from("missing-b.jpg")],
        )
        .await
        .unwrap();

        let meta_id = record_metadata_changed(
            &recorder,
            Some("c"),
            1,
            1,
            "rated.jpg",
            &AssetMetaPatch { rating: Some(1), ..Default::default() },
            &AssetMetaPatch { rating: Some(3), ..Default::default() },
        )
        .await
        .unwrap();

        for id in [
            soft_id, restore_id, purge_id, scan_id, export_id, rebuild_id, tag_add, tag_remove,
            meta_id,
        ] {
            assert!(id > 0);
        }

        let batch_id = record_batch_metadata_changed(
            &recorder,
            Some("c"),
            &[(
                1,
                2,
                "a.jpg".into(),
                AssetMetaPatch { rating: Some(1), ..Default::default() },
                AssetMetaPatch { rating: Some(2), ..Default::default() },
            )],
        )
        .await
        .unwrap();
        assert!(batch_id > 0);
    }

    #[tokio::test]
    async fn record_asset_missing_errors_when_pool_closed() {
        let dir = tempdir().unwrap();
        let catalog = Catalog::open(&dir.path().join("catalog.db")).await.unwrap();
        let pools = catalog.pools().clone();
        let recorder = ActivityRecorder::new(pools.clone());
        pools.close().await;
        assert!(
            record_asset_missing(&recorder, 1, &[String::from("gone.jpg")])
                .await
                .is_err()
        );
    }
}
