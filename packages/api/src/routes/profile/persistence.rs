use crate::entity::profile;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, UpdateMany};

/// Commit against the revision that informed this mutation. This also prevents
/// stale saves from moving the revision backwards after another writer commits.
pub(crate) fn update_profile_revision(
    existing: &profile::Model,
    changes: profile::ActiveModel,
) -> UpdateMany<profile::Entity> {
    profile::Entity::update_many()
        .set(changes)
        .filter(profile::Column::Id.eq(&existing.id))
        .filter(profile::Column::UserId.eq(&existing.user_id))
        .filter(profile::Column::UpdatedAt.eq(existing.updated_at))
        .filter(profile::Column::DeletedAt.is_null())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset};
    use flow_like_types::tokio;
    use sea_orm::{
        ActiveModelTrait, ActiveValue::Set, ConnectionTrait, Database, DatabaseBackend, Statement,
    };
    use serde_json::json;

    fn time(value: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(value).unwrap()
    }

    #[tokio::test]
    #[ignore = "requires FLOW_LIKE_PROFILE_TEST_DATABASE_URL pointing to an empty disposable PostgreSQL database"]
    async fn a_stale_sync_cannot_replace_a_saved_home_or_regress_its_revision() {
        let url = std::env::var("FLOW_LIKE_PROFILE_TEST_DATABASE_URL").unwrap();
        let db = Database::connect(url).await.unwrap();
        let migration =
            include_str!("../../../prisma/migrations-dsql/20260904112415_initial/migration.sql");
        let start = migration.find("CREATE TABLE \"Profile\" (").unwrap();
        let end = migration[start..].find("\n);").unwrap() + start + 3;
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Postgres,
            &migration[start..end],
        ))
        .await
        .unwrap();
        for alteration in include_str!(
            "../../../prisma/migrations-dsql/20260905103000_profile_home_layouts/migration.sql"
        )
        .lines()
        .filter(|line| line.starts_with("ALTER TABLE \"Profile\""))
        {
            db.execute_raw(Statement::from_string(
                DatabaseBackend::Postgres,
                alteration,
            ))
            .await
            .unwrap();
        }

        let original = time("2026-09-07T10:00:00Z");
        let existing = profile::ActiveModel {
            id: Set("workspace".to_owned()),
            user_id: Set("owner".to_owned()),
            name: Set("Workspace".to_owned()),
            hub: Set("api.example.test".to_owned()),
            home_layout: Set(Some(json!({"version": 1, "widgets": []}))),
            created_at: Set(original),
            updated_at: Set(original),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        // Sync has already read this snapshot before a web save commits.
        let mut delayed_sync: profile::ActiveModel = existing.clone().into();
        delayed_sync.home_layout = Set(existing.home_layout.clone());
        delayed_sync.updated_at = Set(time("2026-09-07T10:01:00Z"));

        let saved_layout = json!({"version": 1, "title": "Keep my home", "widgets": []});
        let saved_revision = time("2026-09-07T10:02:00Z");
        let mut web_save: profile::ActiveModel = existing.clone().into();
        web_save.home_layout = Set(Some(saved_layout.clone()));
        web_save.updated_at = Set(saved_revision);
        assert_eq!(
            update_profile_revision(&existing, web_save)
                .exec(&db)
                .await
                .unwrap()
                .rows_affected,
            1
        );
        assert_eq!(
            update_profile_revision(&existing, delayed_sync)
                .exec(&db)
                .await
                .unwrap()
                .rows_affected,
            0
        );
        let latest = profile::Entity::find_by_id((existing.id, existing.user_id))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.home_layout, Some(saved_layout));
        assert_eq!(latest.updated_at, saved_revision);
    }
}
