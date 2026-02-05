use utoipa::{OpenApi, Modify, openapi::security::{SecurityScheme, ApiKey, ApiKeyValue, Http, HttpAuthScheme, OAuth2, Scopes, AuthorizationCode, Flow}};

/// Security scheme modifier to add authentication methods
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);

        // Bearer token (OAuth2 JWT)
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );

        // API Key for technical users (X-API-Key header)
        components.add_security_scheme(
            "api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
        );

        // Personal Access Token (Authorization: PAT <token>)
        components.add_security_scheme(
            "pat",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "Authorization",
                "Personal Access Token. Format: 'PAT <token>'"
            ))),
        );

        // OAuth2 Authorization Code flow
        components.add_security_scheme(
            "oauth2",
            SecurityScheme::OAuth2(OAuth2::new([
                Flow::AuthorizationCode(AuthorizationCode::new(
                    "/api/v1/auth/authorize",
                    "/api/v1/auth/token",
                    Scopes::from_iter([
                        ("openid", "OpenID Connect scope"),
                        ("profile", "User profile information"),
                        ("email", "User email address"),
                    ]),
                ))
            ])),
        );

        // Executor JWT (for backend execution services)
        components.add_security_scheme(
            "executor_jwt",
            SecurityScheme::Http(
                Http::builder()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some("JWT token for execution services"))
                    .build()
            ),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    info(
        title = "Flow-Like API",
        version = "1.0.0",
        description = "Flow-Like platform API for building and executing workflows.\n\n## Authentication\n\nThis API supports multiple authentication methods:\n\n- **Bearer Token (OAuth2)**: Standard JWT token from OAuth2 flow. Use `Authorization: Bearer <token>`\n- **API Key**: For technical/service users. Use `X-API-Key: <key>` header\n- **Personal Access Token (PAT)**: For programmatic access. Use `Authorization: PAT <token>`\n- **Executor JWT**: Internal JWT for execution services",
        license(name = "MIT")
    ),
    servers(
        (url = "/api/v1", description = "API v1")
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication and authorization"),
        (name = "oauth", description = "OAuth provider integration"),
        (name = "user", description = "User management and preferences"),
        (name = "profile", description = "User profiles"),
        (name = "apps", description = "Application management"),
        (name = "boards", description = "Board/workflow management"),
        (name = "pages", description = "Page management"),
        (name = "execution", description = "Workflow execution"),
        (name = "registry", description = "Package registry"),
        (name = "bit", description = "Bit (component) management"),
        (name = "sink", description = "Event sink management"),
        (name = "chat", description = "LLM chat completions"),
        (name = "store", description = "Data store"),
        (name = "solution", description = "Solution requests"),
        (name = "admin", description = "Admin operations"),
        (name = "tmp", description = "Temporary file operations")
    ),
    paths(
        // Health routes
        crate::routes::health::health,
        crate::routes::health::db_health,
        // Auth routes
        crate::routes::auth::openid_config,
        crate::routes::auth::discovery,
        crate::routes::auth::jwks,
        // OAuth routes
        crate::routes::oauth::token_exchange,
        crate::routes::oauth::token_refresh,
        // User routes
        crate::routes::user::info::user_info,
        crate::routes::user::upsert_info::upsert_info,
        crate::routes::user::pricing::get_pricing,
        crate::routes::user::subscribe::create_subscription_checkout,
        crate::routes::user::lookup::user_lookup,
        crate::routes::user::lookup::user_search,
        crate::routes::user::billing::get_billing_session,
        crate::routes::user::notifications::get_notifications,
        crate::routes::user::notifications::list_notifications,
        crate::routes::user::notifications::mark_notification_read,
        crate::routes::user::notifications::mark_all_read,
        crate::routes::user::notifications::delete_notification,
        crate::routes::user::get_invites::get_invites,
        crate::routes::user::manage_invite::accept_invite,
        crate::routes::user::manage_invite::reject_invite,
        crate::routes::user::templates::get_templates,
        crate::routes::user::widgets::get_widgets,
        crate::routes::user::pat::get_pats::get_pats,
        crate::routes::user::pat::create_pat::create_pat,
        crate::routes::user::pat::delete_pat::delete_pat,
        // Profile routes
        crate::routes::profile::get_profiles::get_profiles,
        crate::routes::profile::upsert_profile::upsert_profile,
        crate::routes::profile::delete_profile::delete_profile,
        crate::routes::profile::sync_profiles::sync_profiles,
        // App routes
        crate::routes::app::internal::get_app::get_app,
        crate::routes::app::internal::get_apps::get_apps,
        crate::routes::app::internal::search_apps::search_apps,
        crate::routes::app::internal::upsert_app::upsert_app,
        crate::routes::app::internal::delete_app::delete_app,
        crate::routes::app::internal::change_visibility::change_visibility,
        crate::routes::app::internal::get_nodes::get_nodes,
        // Board routes
        crate::routes::app::board::get_board::get_board,
        crate::routes::app::board::get_boards::get_boards,
        crate::routes::app::board::get_board_versions::get_board_versions,
        crate::routes::app::board::upsert_board::upsert_board,
        crate::routes::app::board::delete_board::delete_board,
        crate::routes::app::board::version_board::version_board,
        crate::routes::app::board::execute_commands::execute_commands,
        crate::routes::app::board::undo_redo_board::undo_board,
        crate::routes::app::board::undo_redo_board::redo_board,
        crate::routes::app::board::invoke_board::invoke_board,
        crate::routes::app::board::invoke_board_async::invoke_board_async,
        crate::routes::app::board::prerun_board::prerun_board,
        crate::routes::app::board::query_logs::query_logs,
        crate::routes::app::board::get_runs::get_runs,
        crate::routes::app::board::get_execution_elements::get_execution_elements,
        // Page routes
        crate::routes::app::page::get_page::get_page,
        crate::routes::app::page::get_pages::get_pages,
        crate::routes::app::page::get_page_by_route::get_page_by_route,
        crate::routes::app::page::upsert_page::upsert_page,
        crate::routes::app::page::delete_page::delete_page,
        // Execution routes
        crate::routes::execution::progress::report_progress,
        crate::routes::execution::progress::push_events,
        crate::routes::execution::progress::poll_status,
        crate::routes::execution::progress::get_run_status,
        crate::routes::execution::public_key::get_execution_jwks,
        // Registry routes
        crate::routes::registry::publish::publish,
        crate::routes::registry::search::search,
        crate::routes::registry::download::download,
        // Bit routes
        crate::routes::bit::get_bit::get_bit,
        crate::routes::bit::get_with_dependencies::get_with_dependencies,
        crate::routes::bit::search_bits::search_bits,
        // Sink routes
        crate::routes::sink::trigger::trigger_http,
        crate::routes::sink::trigger::trigger_telegram,
        crate::routes::sink::trigger::trigger_discord,
        crate::routes::sink::trigger::trigger_service,
        crate::routes::sink::trigger::get_cron_sinks,
        crate::routes::sink::trigger::get_sink_configs,
        crate::routes::sink::management::list_sinks,
        crate::routes::sink::management::list_app_sinks,
        crate::routes::sink::management::get_sink,
        crate::routes::sink::management::update_sink,
        crate::routes::sink::management::toggle_sink,
        // Chat routes
        crate::routes::chat::completions::invoke_llm,
        crate::routes::chat::usage::get_llm_usage,
        // Store routes
        crate::routes::store::get_store_db,
        // Solution routes
        crate::routes::solution::get_upload_url,
        crate::routes::solution::submit_solution,
        crate::routes::solution::track_solution,
        // Tmp routes
        crate::routes::tmp::get_temporary_upload,
        // Admin routes
        crate::routes::admin::solutions::list_solutions::list_solutions,
        crate::routes::admin::packages::get_stats::get_stats,
        crate::routes::admin::packages::get_packages::get_packages,
        crate::routes::admin::packages::get_package::get_package,
        crate::routes::admin::packages::delete_package::delete_package,
        crate::routes::admin::packages::review_package::review_package,
        crate::routes::admin::packages::update_package::update_package,
        crate::routes::admin::bit::delete_bit::delete_bit,
        crate::routes::admin::sinks::list_tokens::list_tokens,
        crate::routes::admin::profiles::delete_profile_template::delete_profile_template,
    ),
    components(schemas(
        // Health schemas
        crate::routes::health::HealthResponse,
        crate::routes::health::DbHealthResponse,
        // OAuth schemas
        crate::routes::oauth::TokenExchangeRequest,
        crate::routes::oauth::TokenRefreshRequest,
        crate::routes::oauth::TokenResponse,
        crate::routes::oauth::ErrorResponse,
    ))
)]
pub struct ApiDoc;
