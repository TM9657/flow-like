use crate::{
    entity::{bit, llm_model, meta, sea_orm_active_enums::BitType},
    error::ApiError,
    middleware::jwt::AppUser,
    routes::LanguageParams,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Query, State},
};
use flow_like::{bit::Bit, hub::BitSearchQuery};
use sea_orm::QueryTrait;
use sea_orm::sea_query::ExprTrait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::{get_bit::temporary_bit, llm_model_to_evaluation};

#[utoipa::path(
    post,
    path = "/bit/search",
    tag = "bit",
    params(
        ("language" = Option<String>, Query, description = "Language code for metadata")
    ),
    request_body = BitSearchQuery,
    responses(
        (status = 200, description = "Search results", body = Vec<Bit>)
    )
)]
#[tracing::instrument(name = "POST /bit", skip(state, user, bit_query, lang_query))]
pub async fn search_bits(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Query(lang_query): Query<LanguageParams>,
    Json(bit_query): Json<BitSearchQuery>,
) -> Result<Json<Vec<Bit>>, ApiError> {
    if !state.platform_config.features.unauthorized_read {
        user.sub()?;
    }

    let language = lang_query.language.as_deref().unwrap_or("en");

    let cache_key = format!("search_bits:v3:{:?}:{:?}", bit_query, language);

    if let Some(cached) = state.get_cache(&cache_key) {
        return Ok(Json(cached));
    }

    let mut bits = search_bits_page(&state.db, &bit_query, language).await?;

    if !state.platform_config.features.unauthorized_read {
        for bit in bits.iter_mut() {
            *bit = temporary_bit(bit.clone(), &state.cdn_bucket).await?;
        }
    }

    state.set_cache(cache_key, &bits);

    Ok(Json(bits))
}

async fn search_bits_page(
    db: &DatabaseConnection,
    bit_query: &BitSearchQuery,
    language: &str,
) -> Result<Vec<Bit>, ApiError> {
    let limit = std::cmp::min(bit_query.limit.unwrap_or(50), 100);
    let mut qb = bit::Entity::find()
        // LIMIT/OFFSET needs a stable order or successive pages can overlap,
        // omitting models from the catalog even when every page is fetched.
        .order_by_asc(bit::Column::Id)
        .limit(Some(limit))
        .offset(bit_query.offset);

    if let Some(types) = &bit_query.bit_types {
        let types: Vec<BitType> = types.iter().cloned().map(Into::into).collect();
        qb = qb.filter(bit::Column::Type.is_in(types));
    }

    if let Some(search_str) = bit_query.search.as_ref() {
        qb = qb.filter(
            bit::Column::Id.in_subquery(
                meta::Entity::find()
                    .select_only()
                    .column(meta::Column::BitId)
                    .filter(meta::Column::BitId.is_not_null())
                    .filter(
                        meta::Column::Description
                            .contains(search_str)
                            .or(meta::Column::Name.contains(search_str)),
                    )
                    .into_query(),
            ),
        );
    }

    // Page model rows before loading metadata. A join here would count each
    // translation toward LIMIT and make a full page look prematurely exhausted.
    let rows: Vec<(bit::Model, Option<llm_model::Model>)> = qb
        .find_also_related(llm_model::Entity)
        .all(db)
        .await
        .map_err(ApiError::from)?;

    let mut bit_positions = std::collections::HashMap::<String, usize>::new();
    let mut bits: Vec<Bit> = rows
        .into_iter()
        .enumerate()
        .map(|(position, (bit_model, llm_model))| {
            bit_positions.insert(bit_model.id.clone(), position);
            let mut bit = Bit::from(bit_model);
            if let Some(llm_model) = llm_model {
                bit.model_evaluation = Some(llm_model_to_evaluation(llm_model));
            }
            bit
        })
        .collect();

    if bits.is_empty() {
        return Ok(bits);
    }

    let metadata = meta::Entity::find()
        .filter(meta::Column::BitId.is_in(bit_positions.keys().cloned()))
        .filter(meta::Column::Lang.is_in([language, "en"]))
        .all(db)
        .await
        .map_err(ApiError::from)?;
    for meta in metadata {
        if let Some(position) = meta.bit_id.as_ref().and_then(|id| bit_positions.get(id)) {
            bits[*position].meta.insert(meta.lang.clone(), meta.into());
        }
    }
    for bit in &mut bits {
        if !bit.meta.contains_key(language)
            && let Some(english) = bit.meta.get("en").cloned()
        {
            bit.meta.insert(language.to_string(), english);
        }
    }
    Ok(bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like::bit::BitTypes;
    use sea_orm::{ConnectOptions, ConnectionTrait, Database, Schema, Set, sea_query::Table};

    #[tokio::test]
    async fn pages_keep_unique_models_with_multiple_or_unavailable_translations() {
        let mut options = ConnectOptions::new("sqlite::memory:");
        options.max_connections(1);
        let db = Database::connect(options).await.unwrap();
        db.execute_unprepared("ATTACH DATABASE ':memory:' AS public")
            .await
            .unwrap();
        let schema = Schema::new(db.get_database_backend());
        for source in [
            schema.create_table_from_entity(bit::Entity),
            schema.create_table_from_entity(llm_model::Entity),
            schema.create_table_from_entity(meta::Entity),
        ] {
            let mut table = Table::create();
            table.table(source.get_table_name().unwrap().clone());
            for column in source.get_columns() {
                table.col(column.clone());
            }
            db.execute(&table).await.unwrap();
        }

        let now = chrono::Utc::now().fixed_offset();
        for index in (0..7).rev() {
            let id = format!("model-{index}");
            bit::Entity::insert(bit::ActiveModel {
                id: Set(id.clone()),
                hub: Set("catalog.example".into()),
                r#type: Set(if index == 6 {
                    BitType::Llm
                } else {
                    BitType::Forecasting
                }),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
            .exec(&db)
            .await
            .unwrap();
            let languages = if index == 1 {
                vec!["fr"]
            } else {
                vec!["en", "de"]
            };
            for language in languages {
                meta::Entity::insert(meta::ActiveModel {
                    id: Set(format!("{id}-{language}")),
                    bit_id: Set(Some(id.clone())),
                    lang: Set(language.into()),
                    name: Set(format!("{id} {language}")),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                })
                .exec(&db)
                .await
                .unwrap();
            }
        }

        let mut all = Vec::new();
        for (offset, expected_count) in [(0, 3), (3, 3), (6, 1)] {
            let page = search_bits_page(
                &db,
                &BitSearchQuery {
                    search: None,
                    limit: Some(3),
                    offset: Some(offset),
                    bit_types: Some(vec![BitTypes::Forecasting, BitTypes::Llm]),
                },
                "de",
            )
            .await
            .unwrap();
            assert_eq!(page.len(), expected_count);
            all.extend(page);
        }
        assert_eq!(
            all.iter().map(|bit| bit.id.as_str()).collect::<Vec<_>>(),
            (0..7)
                .map(|index| format!("model-{index}"))
                .collect::<Vec<_>>()
        );
        assert_eq!(all[0].meta["de"].name, "model-0 de");
        assert_eq!(all[0].meta["en"].name, "model-0 en");
        assert!(all[1].meta.is_empty());

        let filtered = search_bits_page(
            &db,
            &BitSearchQuery {
                search: Some("model-2".into()),
                bit_types: Some(vec![BitTypes::Forecasting]),
                ..BitSearchQuery::builder()
            },
            "es",
        )
        .await
        .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "model-2");
        assert_eq!(filtered[0].meta["es"].name, "model-2 en");
    }
}
