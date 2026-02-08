//! Server-side WASM Package Registry
//!
//! Stores WASM binaries in CDN/content bucket and metadata in PostgreSQL.
//! Custom nodes are public after admin approval.

use crate::entity::{
    user, wasm_package, wasm_package_author, wasm_package_review, wasm_package_version,
};
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_storage::object_store::PutPayload;
use flow_like_storage::object_store::path::Path;
use flow_like_types::create_id;
use flow_like_wasm::manifest::PackageManifest;
use flow_like_wasm::registry::{
    PackageStatus, PackageSummary, PackageVersion, PublishResponse, RegistryEntry, RegistryIndex,
    SearchFilters, SearchResults, SortField,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use utoipa::ToSchema;

/// CDN path prefix for WASM packages
const WASM_PACKAGES_PATH: &str = "wasm-packages";

/// Author information for display
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthorInfo {
    pub user_id: String,
    pub username: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub role: Option<String>,
}

/// Extended package summary with additional metadata for UI
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PackageDetails {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub authors: Vec<AuthorInfo>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub status: String,
    pub verified: bool,
    pub download_count: u64,
    pub wasm_size: u64,
    pub nodes: serde_json::Value,
    pub permissions: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Package review entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PackageReview {
    pub id: String,
    pub package_id: String,
    pub reviewer_id: String,
    pub action: String,
    pub comment: Option<String>,
    pub security_score: Option<i32>,
    pub code_quality_score: Option<i32>,
    pub documentation_score: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Request to submit a review
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewRequest {
    pub action: String, // "approve", "reject", "request_changes", "comment", "flag"
    pub comment: Option<String>,
    pub internal_note: Option<String>,
    pub security_score: Option<i32>,
    pub code_quality_score: Option<i32>,
    pub documentation_score: Option<i32>,
}

/// Statistics for the registry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegistryStats {
    pub total_packages: i64,
    pub total_versions: i64,
    pub total_downloads: i64,
    pub pending_review: i64,
    pub active_packages: i64,
    pub rejected_packages: i64,
    pub verified_packages: i64,
}

fn status_to_string(status: &crate::entity::sea_orm_active_enums::WasmPackageStatus) -> String {
    use crate::entity::sea_orm_active_enums::WasmPackageStatus;
    match status {
        WasmPackageStatus::PendingReview => "pending_review".to_string(),
        WasmPackageStatus::Active => "active".to_string(),
        WasmPackageStatus::Rejected => "rejected".to_string(),
        WasmPackageStatus::Deprecated => "deprecated".to_string(),
        WasmPackageStatus::Disabled => "disabled".to_string(),
    }
}

fn status_to_enum(status: &str) -> crate::entity::sea_orm_active_enums::WasmPackageStatus {
    use crate::entity::sea_orm_active_enums::WasmPackageStatus;
    match status {
        "pending_review" => WasmPackageStatus::PendingReview,
        "active" => WasmPackageStatus::Active,
        "rejected" => WasmPackageStatus::Rejected,
        "deprecated" => WasmPackageStatus::Deprecated,
        "disabled" => WasmPackageStatus::Disabled,
        _ => WasmPackageStatus::PendingReview,
    }
}

/// Server-side registry for managing WASM packages
/// Uses PostgreSQL for metadata and CDN for WASM binaries
pub struct ServerRegistry {
    db: DatabaseConnection,
    cdn_bucket: Arc<FlowLikeStore>,
    cdn_base_url: Option<String>,
}

impl ServerRegistry {
    pub fn new(
        db: DatabaseConnection,
        cdn_bucket: Arc<FlowLikeStore>,
        cdn_base_url: Option<String>,
    ) -> Self {
        Self {
            db,
            cdn_bucket,
            cdn_base_url,
        }
    }

    /// Get storage path for a WASM package version
    fn wasm_path(package_id: &str, version: &str) -> Path {
        Path::from(WASM_PACKAGES_PATH)
            .child(package_id)
            .child(format!("{}.wasm", version))
    }

    /// Get a signed URL or CDN URL for downloading a WASM file
    async fn get_download_url(
        &self,
        package_id: &str,
        version: &str,
    ) -> flow_like_types::Result<String> {
        let path = Self::wasm_path(package_id, version);

        // If CDN is configured, return direct CDN URL (public access)
        if let Some(cdn_url) = &self.cdn_base_url {
            return Ok(format!("{}/{}", cdn_url, path));
        }

        // Otherwise generate a signed URL (valid for 1 hour)
        let url = self
            .cdn_bucket
            .sign("GET", &path, Duration::from_secs(3600))
            .await?;
        Ok(url.to_string())
    }

    /// Get the registry index (list of all active packages - public API)
    pub async fn get_index(&self) -> flow_like_types::Result<RegistryIndex> {
        use crate::entity::sea_orm_active_enums::WasmPackageStatus;

        let packages = wasm_package::Entity::find()
            .filter(wasm_package::Column::Status.eq(WasmPackageStatus::Active))
            .order_by_desc(wasm_package::Column::DownloadCount)
            .all(&self.db)
            .await?;

        let summaries: Vec<PackageSummary> = packages
            .into_iter()
            .map(|pkg| PackageSummary {
                id: pkg.id,
                name: pkg.name,
                description: pkg.description,
                latest_version: pkg.version,
                download_count: pkg.download_count as u64,
                status: PackageStatus::Active,
                keywords: pkg.keywords,
                verified: pkg.verified,
            })
            .collect();

        Ok(RegistryIndex {
            name: "Flow-Like WASM Registry".to_string(),
            url: String::new(),
            updated_at: chrono::Utc::now(),
            packages: summaries,
        })
    }

    /// Get registry statistics (admin)
    pub async fn get_stats(&self) -> flow_like_types::Result<RegistryStats> {
        use crate::entity::sea_orm_active_enums::WasmPackageStatus;

        let total_packages = wasm_package::Entity::find().count(&self.db).await? as i64;

        let total_versions = wasm_package_version::Entity::find().count(&self.db).await? as i64;

        let active_packages = wasm_package::Entity::find()
            .filter(wasm_package::Column::Status.eq(WasmPackageStatus::Active))
            .count(&self.db)
            .await? as i64;

        let pending_review = wasm_package::Entity::find()
            .filter(wasm_package::Column::Status.eq(WasmPackageStatus::PendingReview))
            .count(&self.db)
            .await? as i64;

        let rejected_packages = wasm_package::Entity::find()
            .filter(wasm_package::Column::Status.eq(WasmPackageStatus::Rejected))
            .count(&self.db)
            .await? as i64;

        let verified_packages = wasm_package::Entity::find()
            .filter(wasm_package::Column::Verified.eq(true))
            .count(&self.db)
            .await? as i64;

        // Sum all download counts
        let downloads_result: Option<i64> = wasm_package::Entity::find()
            .select_only()
            .column_as(wasm_package::Column::DownloadCount.sum(), "total")
            .into_tuple()
            .one(&self.db)
            .await?;

        Ok(RegistryStats {
            total_packages,
            total_versions,
            total_downloads: downloads_result.unwrap_or(0),
            pending_review,
            active_packages,
            rejected_packages,
            verified_packages,
        })
    }

    /// Get a package entry by ID (public - only returns active packages)
    pub async fn get_package(&self, id: &str) -> flow_like_types::Result<Option<RegistryEntry>> {
        use crate::entity::sea_orm_active_enums::WasmPackageStatus;

        let Some(pkg) = wasm_package::Entity::find_by_id(id)
            .filter(wasm_package::Column::Status.eq(WasmPackageStatus::Active))
            .one(&self.db)
            .await?
        else {
            return Ok(None);
        };

        self.build_registry_entry(pkg).await.map(Some)
    }

    /// Get a package entry by ID (admin - returns any status)
    pub async fn get_package_admin(
        &self,
        id: &str,
    ) -> flow_like_types::Result<Option<PackageDetails>> {
        let Some(pkg) = wasm_package::Entity::find_by_id(id).one(&self.db).await? else {
            return Ok(None);
        };

        let authors = self.get_package_authors(&pkg.id).await?;

        Ok(Some(PackageDetails {
            id: pkg.id,
            name: pkg.name,
            description: pkg.description,
            version: pkg.version,
            authors,
            license: pkg.license,
            homepage: pkg.homepage,
            repository: pkg.repository,
            keywords: pkg.keywords,
            status: status_to_string(&pkg.status),
            verified: pkg.verified,
            download_count: pkg.download_count as u64,
            wasm_size: pkg.wasm_size as u64,
            nodes: pkg.nodes,
            permissions: pkg.permissions,
            created_at: chrono::DateTime::from_naive_utc_and_offset(pkg.created_at, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(pkg.updated_at, chrono::Utc),
            published_at: pkg
                .published_at
                .map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
        }))
    }

    /// Get authors for a package with user information
    async fn get_package_authors(
        &self,
        package_id: &str,
    ) -> flow_like_types::Result<Vec<AuthorInfo>> {
        let author_records = wasm_package_author::Entity::find()
            .filter(wasm_package_author::Column::PackageId.eq(package_id))
            .all(&self.db)
            .await?;

        let mut authors = Vec::new();
        for record in author_records {
            let user_info = user::Entity::find_by_id(&record.user_id)
                .one(&self.db)
                .await?;

            authors.push(AuthorInfo {
                user_id: record.user_id,
                username: user_info.as_ref().and_then(|u| u.username.clone()),
                name: user_info.as_ref().and_then(|u| u.name.clone()),
                avatar: user_info.and_then(|u| u.avatar),
                role: record.role,
            });
        }

        Ok(authors)
    }

    /// Build a RegistryEntry from a package model
    async fn build_registry_entry(
        &self,
        pkg: wasm_package::Model,
    ) -> flow_like_types::Result<RegistryEntry> {
        use crate::entity::sea_orm_active_enums::WasmPackageStatus;

        let versions = wasm_package_version::Entity::find()
            .filter(wasm_package_version::Column::PackageId.eq(&pkg.id))
            .order_by_desc(wasm_package_version::Column::PublishedAt)
            .all(&self.db)
            .await?;

        // Get authors from junction table
        let author_infos = self.get_package_authors(&pkg.id).await?;
        let authors: Vec<flow_like_wasm::manifest::PackageAuthor> = author_infos
            .into_iter()
            .map(|a| flow_like_wasm::manifest::PackageAuthor {
                name: a.name.or(a.username).unwrap_or(a.user_id),
                email: None,
                url: None,
            })
            .collect();

        let manifest = PackageManifest {
            manifest_version: flow_like_wasm::manifest::MANIFEST_VERSION,
            id: pkg.id.clone(),
            name: pkg.name.clone(),
            description: pkg.description.clone(),
            version: pkg.version.clone(),
            authors,
            license: pkg.license.clone(),
            homepage: pkg.homepage.clone(),
            repository: pkg.repository.clone(),
            keywords: pkg.keywords.clone(),
            nodes: serde_json::from_value(pkg.nodes.clone()).unwrap_or_default(),
            permissions: serde_json::from_value(pkg.permissions.clone()).unwrap_or_default(),
            min_flow_like_version: None,
            wasm_path: Some(pkg.wasm_path.clone()),
            wasm_hash: Some(pkg.wasm_hash.clone()),
            metadata: Default::default(),
        };

        let package_versions: Vec<PackageVersion> = versions
            .into_iter()
            .map(|v| PackageVersion {
                version: v.version,
                wasm_hash: v.wasm_hash,
                wasm_size: v.wasm_size as u64,
                download_url: None,
                published_at: chrono::DateTime::from_naive_utc_and_offset(
                    v.published_at,
                    chrono::Utc,
                ),
                min_flow_like_version: v.min_flow_like_version,
                release_notes: v.release_notes,
                yanked: v.yanked,
            })
            .collect();

        Ok(RegistryEntry {
            id: pkg.id,
            manifest,
            versions: package_versions,
            status: match pkg.status {
                WasmPackageStatus::Active => PackageStatus::Active,
                WasmPackageStatus::Deprecated => PackageStatus::Deprecated,
                _ => PackageStatus::Disabled,
            },
            download_count: pkg.download_count as u64,
            created_at: chrono::DateTime::from_naive_utc_and_offset(pkg.created_at, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(pkg.updated_at, chrono::Utc),
            source: flow_like_wasm::registry::PackageSource::Remote {
                registry_url: String::new(),
                download_url: String::new(),
            },
            verified: pkg.verified,
        })
    }

    /// Search packages with filters
    pub async fn search(&self, filters: &SearchFilters) -> flow_like_types::Result<SearchResults> {
        use crate::entity::sea_orm_active_enums::WasmPackageStatus;

        let mut query = wasm_package::Entity::find();

        // Only show active packages unless including deprecated
        if !filters.include_deprecated {
            query = query.filter(wasm_package::Column::Status.eq(WasmPackageStatus::Active));
        }

        // Filter by verified only
        if filters.verified_only {
            query = query.filter(wasm_package::Column::Verified.eq(true));
        }

        // Text search (name, description, keywords)
        if let Some(q) = &filters.query {
            let pattern = format!("%{}%", q.to_lowercase());
            query = query.filter(
                wasm_package::Column::Name
                    .contains(&pattern)
                    .or(wasm_package::Column::Description.contains(&pattern))
                    .or(wasm_package::Column::Id.contains(&pattern)),
            );
        }

        // Get total count before pagination
        let total_count = query.clone().count(&self.db).await? as usize;

        // Apply sorting
        query = match filters.sort_by {
            SortField::Downloads => {
                if filters.sort_desc {
                    query.order_by_desc(wasm_package::Column::DownloadCount)
                } else {
                    query.order_by_asc(wasm_package::Column::DownloadCount)
                }
            }
            SortField::Name => {
                if filters.sort_desc {
                    query.order_by_desc(wasm_package::Column::Name)
                } else {
                    query.order_by_asc(wasm_package::Column::Name)
                }
            }
            SortField::UpdatedAt => {
                if filters.sort_desc {
                    query.order_by_desc(wasm_package::Column::UpdatedAt)
                } else {
                    query.order_by_asc(wasm_package::Column::UpdatedAt)
                }
            }
            SortField::CreatedAt => {
                if filters.sort_desc {
                    query.order_by_desc(wasm_package::Column::CreatedAt)
                } else {
                    query.order_by_asc(wasm_package::Column::CreatedAt)
                }
            }
            SortField::Relevance => {
                // For relevance, sort by downloads as a proxy for popularity
                if filters.sort_desc {
                    query.order_by_desc(wasm_package::Column::DownloadCount)
                } else {
                    query.order_by_asc(wasm_package::Column::DownloadCount)
                }
            }
        };

        // Apply pagination
        let packages = query
            .offset(filters.offset as u64)
            .limit(filters.limit as u64)
            .all(&self.db)
            .await?;

        let summaries: Vec<PackageSummary> = packages
            .into_iter()
            .map(|pkg| PackageSummary {
                id: pkg.id,
                name: pkg.name,
                description: pkg.description,
                latest_version: pkg.version,
                download_count: pkg.download_count as u64,
                status: match pkg.status {
                    WasmPackageStatus::Active => PackageStatus::Active,
                    WasmPackageStatus::Deprecated => PackageStatus::Deprecated,
                    _ => PackageStatus::Disabled,
                },
                keywords: pkg.keywords,
                verified: pkg.verified,
            })
            .collect();

        Ok(SearchResults {
            packages: summaries,
            total_count,
            offset: filters.offset,
            limit: filters.limit,
        })
    }

    /// Get download URL for a package (signed URL or CDN URL)
    pub async fn get_wasm_url(
        &self,
        package_id: &str,
        version: Option<&str>,
    ) -> flow_like_types::Result<(String, PackageManifest, String)> {
        let Some(entry) = self.get_package(package_id).await? else {
            return Err(flow_like_types::anyhow!(
                "Package not found: {}",
                package_id
            ));
        };

        let ver = if let Some(v) = version {
            entry
                .get_version(v)
                .ok_or_else(|| flow_like_types::anyhow!("Version not found: {}", v))?
                .clone()
        } else {
            entry
                .latest_version()
                .ok_or_else(|| flow_like_types::anyhow!("No versions available"))?
                .clone()
        };

        let download_url = self.get_download_url(package_id, &ver.version).await?;

        Ok((download_url, entry.manifest, ver.version))
    }

    /// Download package WASM binary directly (for backward compatibility)
    pub async fn download_wasm(
        &self,
        package_id: &str,
        version: Option<&str>,
    ) -> flow_like_types::Result<(Vec<u8>, PackageManifest, String)> {
        let Some(entry) = self.get_package(package_id).await? else {
            return Err(flow_like_types::anyhow!(
                "Package not found: {}",
                package_id
            ));
        };

        let ver = if let Some(v) = version {
            entry
                .get_version(v)
                .ok_or_else(|| flow_like_types::anyhow!("Version not found: {}", v))?
                .clone()
        } else {
            entry
                .latest_version()
                .ok_or_else(|| flow_like_types::anyhow!("No versions available"))?
                .clone()
        };

        let path = Self::wasm_path(package_id, &ver.version);
        let data = self.cdn_bucket.as_generic().get(&path).await?;
        let bytes = data.bytes().await?.to_vec();

        Ok((bytes, entry.manifest, ver.version))
    }

    /// Publish a new package or version (sets to pending review)
    pub async fn publish(
        &self,
        manifest: PackageManifest,
        wasm_data: Vec<u8>,
        submitter_id: Option<String>,
        _submitter_email: Option<String>,
    ) -> flow_like_types::Result<PublishResponse> {
        use crate::entity::sea_orm_active_enums::{WasmPackageStatus, WasmReviewAction};

        let now = chrono::Utc::now().naive_utc();
        let hash = blake3::hash(&wasm_data).to_hex().to_string();
        let size = wasm_data.len() as i64;

        // Check if version already exists
        let existing_version = wasm_package_version::Entity::find()
            .filter(wasm_package_version::Column::PackageId.eq(&manifest.id))
            .filter(wasm_package_version::Column::Version.eq(&manifest.version))
            .one(&self.db)
            .await?;

        if existing_version.is_some() {
            return Err(flow_like_types::anyhow!(
                "Version {} already exists for package {}",
                manifest.version,
                manifest.id
            ));
        }

        // Upload WASM to CDN bucket
        let wasm_path = Self::wasm_path(&manifest.id, &manifest.version);
        self.cdn_bucket
            .as_generic()
            .put(&wasm_path, PutPayload::from(wasm_data))
            .await?;

        // Check if package exists
        let existing_package = wasm_package::Entity::find_by_id(&manifest.id)
            .one(&self.db)
            .await?;

        if let Some(_existing) = existing_package {
            // Update existing package with new version info (keeps current status)
            let update_model = wasm_package::ActiveModel {
                id: Set(manifest.id.clone()),
                name: Set(manifest.name.clone()),
                description: Set(manifest.description.clone()),
                version: Set(manifest.version.clone()),
                license: Set(manifest.license.clone()),
                homepage: Set(manifest.homepage.clone()),
                repository: Set(manifest.repository.clone()),
                keywords: Set(manifest.keywords.clone()),
                wasm_path: Set(wasm_path.to_string()),
                wasm_hash: Set(hash.clone()),
                wasm_size: Set(size),
                nodes: Set(serde_json::to_value(&manifest.nodes)?),
                permissions: Set(serde_json::to_value(&manifest.permissions)?),
                updated_at: Set(now),
                ..Default::default()
            };
            update_model.update(&self.db).await?;
        } else {
            // Create new package with PENDING_REVIEW status
            let package_model = wasm_package::ActiveModel {
                id: Set(manifest.id.clone()),
                name: Set(manifest.name.clone()),
                description: Set(manifest.description.clone()),
                version: Set(manifest.version.clone()),
                license: Set(manifest.license.clone()),
                homepage: Set(manifest.homepage.clone()),
                repository: Set(manifest.repository.clone()),
                keywords: Set(manifest.keywords.clone()),
                status: Set(WasmPackageStatus::PendingReview),
                verified: Set(false),
                download_count: Set(0),
                wasm_path: Set(wasm_path.to_string()),
                wasm_hash: Set(hash.clone()),
                wasm_size: Set(size),
                nodes: Set(serde_json::to_value(&manifest.nodes)?),
                permissions: Set(serde_json::to_value(&manifest.permissions)?),
                created_at: Set(now),
                updated_at: Set(now),
                published_at: Set(None),
            };
            package_model.insert(&self.db).await?;

            // Add submitter as creator author if they have a user ID
            if let Some(ref user_id) = submitter_id {
                let author_model = wasm_package_author::ActiveModel {
                    id: Set(create_id()),
                    package_id: Set(manifest.id.clone()),
                    user_id: Set(user_id.clone()),
                    role: Set(Some("creator".to_string())),
                    added_at: Set(now),
                };
                author_model.insert(&self.db).await?;
            }

            // Create initial review record
            let review_model = wasm_package_review::ActiveModel {
                id: Set(create_id()),
                package_id: Set(manifest.id.clone()),
                reviewer_id: Set(submitter_id
                    .clone()
                    .unwrap_or_else(|| "anonymous".to_string())),
                action: Set(WasmReviewAction::Submitted),
                comment: Set(None),
                internal_note: Set(None),
                security_score: Set(None),
                code_quality_score: Set(None),
                documentation_score: Set(None),
                created_at: Set(now),
            };
            review_model.insert(&self.db).await?;
        }

        // Create version record with PENDING_REVIEW status
        let version_model = wasm_package_version::ActiveModel {
            id: Set(create_id()),
            package_id: Set(manifest.id.clone()),
            version: Set(manifest.version.clone()),
            wasm_path: Set(wasm_path.to_string()),
            wasm_hash: Set(hash),
            wasm_size: Set(size),
            release_notes: Set(None),
            min_flow_like_version: Set(None),
            yanked: Set(false),
            status: Set(WasmPackageStatus::PendingReview),
            published_at: Set(now),
            approved_at: Set(None),
        };
        version_model.insert(&self.db).await?;

        Ok(PublishResponse {
            success: true,
            package_id: manifest.id,
            version: manifest.version,
            message: Some(
                "Package submitted for review. An admin will review it shortly.".to_string(),
            ),
        })
    }

    /// Increment download count for a package (fire and forget)
    pub async fn increment_downloads(&self, package_id: &str) -> flow_like_types::Result<()> {
        // Use raw SQL for atomic increment
        sea_orm::ConnectionTrait::execute(
            &self.db,
            sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                r#"UPDATE "WasmPackage" SET "downloadCount" = "downloadCount" + 1 WHERE id = $1"#,
                [package_id.into()],
            ),
        )
        .await?;
        Ok(())
    }

    /// Get all versions for a package
    pub async fn get_versions(
        &self,
        package_id: &str,
    ) -> flow_like_types::Result<Vec<PackageVersion>> {
        let versions = wasm_package_version::Entity::find()
            .filter(wasm_package_version::Column::PackageId.eq(package_id))
            .order_by_desc(wasm_package_version::Column::PublishedAt)
            .all(&self.db)
            .await?;

        Ok(versions
            .into_iter()
            .map(|v| PackageVersion {
                version: v.version,
                wasm_hash: v.wasm_hash,
                wasm_size: v.wasm_size as u64,
                download_url: None,
                published_at: chrono::DateTime::from_naive_utc_and_offset(
                    v.published_at,
                    chrono::Utc,
                ),
                min_flow_like_version: v.min_flow_like_version,
                release_notes: v.release_notes,
                yanked: v.yanked,
            })
            .collect())
    }

    // ==================== ADMIN METHODS ====================

    /// List packages for admin (includes all statuses)
    pub async fn list_packages_admin(
        &self,
        status_filter: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> flow_like_types::Result<(Vec<PackageDetails>, usize)> {
        let mut query = wasm_package::Entity::find();

        if let Some(status) = status_filter {
            query = query.filter(wasm_package::Column::Status.eq(status_to_enum(status)));
        }

        let total_count = query.clone().count(&self.db).await? as usize;

        let packages = query
            .order_by_desc(wasm_package::Column::CreatedAt)
            .offset(offset as u64)
            .limit(limit as u64)
            .all(&self.db)
            .await?;

        let mut details: Vec<PackageDetails> = Vec::with_capacity(packages.len());
        for pkg in packages {
            let authors = self.get_package_authors(&pkg.id).await?;
            details.push(PackageDetails {
                id: pkg.id,
                name: pkg.name,
                description: pkg.description,
                version: pkg.version,
                authors,
                license: pkg.license,
                homepage: pkg.homepage,
                repository: pkg.repository,
                keywords: pkg.keywords,
                status: status_to_string(&pkg.status),
                verified: pkg.verified,
                download_count: pkg.download_count as u64,
                wasm_size: pkg.wasm_size as u64,
                nodes: pkg.nodes,
                permissions: pkg.permissions,
                created_at: chrono::DateTime::from_naive_utc_and_offset(
                    pkg.created_at,
                    chrono::Utc,
                ),
                updated_at: chrono::DateTime::from_naive_utc_and_offset(
                    pkg.updated_at,
                    chrono::Utc,
                ),
                published_at: pkg
                    .published_at
                    .map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)),
            });
        }

        Ok((details, total_count))
    }

    /// Submit a review for a package
    pub async fn submit_review(
        &self,
        package_id: &str,
        reviewer_id: &str,
        review: ReviewRequest,
    ) -> flow_like_types::Result<PackageReview> {
        use crate::entity::sea_orm_active_enums::{WasmPackageStatus, WasmReviewAction};

        let now = chrono::Utc::now().naive_utc();

        // Verify package exists
        let Some(_pkg) = wasm_package::Entity::find_by_id(package_id)
            .one(&self.db)
            .await?
        else {
            return Err(flow_like_types::anyhow!(
                "Package not found: {}",
                package_id
            ));
        };

        let action = match review.action.as_str() {
            "approve" => WasmReviewAction::Approved,
            "reject" => WasmReviewAction::Rejected,
            "request_changes" => WasmReviewAction::RequestedChanges,
            "comment" => WasmReviewAction::Commented,
            "flag" => WasmReviewAction::Flagged,
            _ => {
                return Err(flow_like_types::anyhow!(
                    "Invalid review action: {}",
                    review.action
                ));
            }
        };

        // Update package status based on action
        let new_status = match action {
            WasmReviewAction::Approved => Some(WasmPackageStatus::Active),
            WasmReviewAction::Rejected => Some(WasmPackageStatus::Rejected),
            _ => None,
        };

        if let Some(status) = new_status {
            let mut update_model = wasm_package::ActiveModel {
                id: Set(package_id.to_string()),
                status: Set(status),
                updated_at: Set(now),
                ..Default::default()
            };

            if status == WasmPackageStatus::Active {
                update_model.published_at = Set(Some(now));
            }

            update_model.update(&self.db).await?;
        }

        // Create review record
        let review_id = create_id();
        let review_model = wasm_package_review::ActiveModel {
            id: Set(review_id.clone()),
            package_id: Set(package_id.to_string()),
            reviewer_id: Set(reviewer_id.to_string()),
            action: Set(action),
            comment: Set(review.comment.clone()),
            internal_note: Set(review.internal_note),
            security_score: Set(review.security_score),
            code_quality_score: Set(review.code_quality_score),
            documentation_score: Set(review.documentation_score),
            created_at: Set(now),
        };
        review_model.insert(&self.db).await?;

        Ok(PackageReview {
            id: review_id,
            package_id: package_id.to_string(),
            reviewer_id: reviewer_id.to_string(),
            action: review.action,
            comment: review.comment,
            security_score: review.security_score,
            code_quality_score: review.code_quality_score,
            documentation_score: review.documentation_score,
            created_at: chrono::DateTime::from_naive_utc_and_offset(now, chrono::Utc),
        })
    }

    /// Get reviews for a package
    pub async fn get_reviews(
        &self,
        package_id: &str,
    ) -> flow_like_types::Result<Vec<PackageReview>> {
        let reviews = wasm_package_review::Entity::find()
            .filter(wasm_package_review::Column::PackageId.eq(package_id))
            .order_by_desc(wasm_package_review::Column::CreatedAt)
            .all(&self.db)
            .await?;

        Ok(reviews
            .into_iter()
            .map(|r| {
                let action_str = match r.action {
                    crate::entity::sea_orm_active_enums::WasmReviewAction::Submitted => "submitted",
                    crate::entity::sea_orm_active_enums::WasmReviewAction::Approved => "approve",
                    crate::entity::sea_orm_active_enums::WasmReviewAction::Rejected => "reject",
                    crate::entity::sea_orm_active_enums::WasmReviewAction::RequestedChanges => {
                        "request_changes"
                    }
                    crate::entity::sea_orm_active_enums::WasmReviewAction::Commented => "comment",
                    crate::entity::sea_orm_active_enums::WasmReviewAction::Flagged => "flag",
                };
                PackageReview {
                    id: r.id,
                    package_id: r.package_id,
                    reviewer_id: r.reviewer_id,
                    action: action_str.to_string(),
                    comment: r.comment,
                    security_score: r.security_score,
                    code_quality_score: r.code_quality_score,
                    documentation_score: r.documentation_score,
                    created_at: chrono::DateTime::from_naive_utc_and_offset(
                        r.created_at,
                        chrono::Utc,
                    ),
                }
            })
            .collect())
    }

    /// Update package status directly (admin)
    pub async fn update_status(
        &self,
        package_id: &str,
        status: &str,
        verified: Option<bool>,
    ) -> flow_like_types::Result<()> {
        let now = chrono::Utc::now().naive_utc();
        let new_status = status_to_enum(status);

        let mut update_model = wasm_package::ActiveModel {
            id: Set(package_id.to_string()),
            status: Set(new_status),
            updated_at: Set(now),
            ..Default::default()
        };

        if let Some(v) = verified {
            update_model.verified = Set(v);
        }

        if new_status == crate::entity::sea_orm_active_enums::WasmPackageStatus::Active {
            update_model.published_at = Set(Some(now));
        }

        update_model.update(&self.db).await?;
        Ok(())
    }

    /// Delete a package (admin)
    pub async fn delete_package(&self, package_id: &str) -> flow_like_types::Result<()> {
        // Delete WASM files from CDN
        let versions = wasm_package_version::Entity::find()
            .filter(wasm_package_version::Column::PackageId.eq(package_id))
            .all(&self.db)
            .await?;

        for version in versions {
            let path = Self::wasm_path(package_id, &version.version);
            let _ = self.cdn_bucket.as_generic().delete(&path).await;
        }

        // Delete from database (cascade will handle versions and reviews)
        wasm_package::Entity::delete_by_id(package_id)
            .exec(&self.db)
            .await?;

        Ok(())
    }

    // ==================== AUTHOR MANAGEMENT ====================

    /// Add an author to a package
    pub async fn add_author(
        &self,
        package_id: &str,
        user_id: &str,
        role: Option<String>,
    ) -> flow_like_types::Result<AuthorInfo> {
        let now = chrono::Utc::now().naive_utc();

        // Verify package exists
        let Some(_pkg) = wasm_package::Entity::find_by_id(package_id)
            .one(&self.db)
            .await?
        else {
            return Err(flow_like_types::anyhow!(
                "Package not found: {}",
                package_id
            ));
        };

        // Verify user exists
        let Some(user_record) = user::Entity::find_by_id(user_id).one(&self.db).await? else {
            return Err(flow_like_types::anyhow!("User not found: {}", user_id));
        };

        // Check if already an author
        let existing = wasm_package_author::Entity::find()
            .filter(wasm_package_author::Column::PackageId.eq(package_id))
            .filter(wasm_package_author::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?;

        if existing.is_some() {
            return Err(flow_like_types::anyhow!(
                "User is already an author of this package"
            ));
        }

        let author_model = wasm_package_author::ActiveModel {
            id: Set(create_id()),
            package_id: Set(package_id.to_string()),
            user_id: Set(user_id.to_string()),
            role: Set(role.clone()),
            added_at: Set(now),
        };
        author_model.insert(&self.db).await?;

        Ok(AuthorInfo {
            user_id: user_id.to_string(),
            username: user_record.username,
            name: user_record.name,
            avatar: user_record.avatar,
            role,
        })
    }

    /// Remove an author from a package
    pub async fn remove_author(
        &self,
        package_id: &str,
        user_id: &str,
    ) -> flow_like_types::Result<()> {
        let result = wasm_package_author::Entity::delete_many()
            .filter(wasm_package_author::Column::PackageId.eq(package_id))
            .filter(wasm_package_author::Column::UserId.eq(user_id))
            .exec(&self.db)
            .await?;

        if result.rows_affected == 0 {
            return Err(flow_like_types::anyhow!(
                "Author not found for this package"
            ));
        }

        Ok(())
    }

    /// Get packages authored by a user
    pub async fn get_user_packages(
        &self,
        user_id: &str,
    ) -> flow_like_types::Result<Vec<PackageSummary>> {
        let author_records = wasm_package_author::Entity::find()
            .filter(wasm_package_author::Column::UserId.eq(user_id))
            .all(&self.db)
            .await?;

        let package_ids: Vec<String> = author_records.into_iter().map(|a| a.package_id).collect();

        if package_ids.is_empty() {
            return Ok(Vec::new());
        }

        let packages = wasm_package::Entity::find()
            .filter(wasm_package::Column::Id.is_in(package_ids))
            .all(&self.db)
            .await?;

        Ok(packages
            .into_iter()
            .map(|pkg| PackageSummary {
                id: pkg.id,
                name: pkg.name,
                description: pkg.description,
                latest_version: pkg.version,
                download_count: pkg.download_count as u64,
                status: match pkg.status {
                    crate::entity::sea_orm_active_enums::WasmPackageStatus::Active => {
                        PackageStatus::Active
                    }
                    crate::entity::sea_orm_active_enums::WasmPackageStatus::Deprecated => {
                        PackageStatus::Deprecated
                    }
                    _ => PackageStatus::Disabled,
                },
                keywords: pkg.keywords,
                verified: pkg.verified,
            })
            .collect())
    }
}
