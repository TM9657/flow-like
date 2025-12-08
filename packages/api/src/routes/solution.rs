use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use chrono::{Duration as ChronoDuration, Utc};
use flow_like_storage::Path as FLPath;
use flow_like_types::tokio::try_join;
use flow_like_types::{
    create_id,
    mime_guess::{self, mime},
};
use mime::Mime;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const UPLOAD_TTL_SECS: u64 = 60 * 60; // 1 hour
const DOWNLOAD_TTL_SECS: u64 = 60 * 60 * 24 * 7; // 7 days
const SIZE_LIMIT_BYTES: Option<u64> = Some(1024 * 1024 * 50); // 50 MB

const DEPOSIT_AMOUNT_CENTS: i64 = 50000; // €500 deposit for priority queue
const STANDARD_TOTAL_CENTS: i64 = 240000; // €2,400 total
const APPSTORE_TOTAL_CENTS: i64 = 199900; // €1,999 total

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SolutionUploadResponse {
    key: String,
    content_type: String,
    upload_url: String,
    upload_expires_at: String,
    download_url: String,
    download_expires_at: String,
    size_limit_bytes: Option<u64>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SolutionSubmission {
    name: String,
    email: String,
    company: String,
    application_type: String,
    data_security: String,
    description: String,
    example_input: String,
    expected_output: String,
    files: Vec<UploadedFile>,
    user_count: String,
    user_type: String,
    technical_level: String,
    timeline: Option<String>,
    additional_notes: Option<String>,
    pricing_tier: String,
    pay_deposit: bool,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFile {
    name: String,
    key: String,
    download_url: String,
    size: u64,
}

#[derive(Deserialize, Debug)]
pub struct UploadParams {
    extension: Option<String>,
    content_type: Option<String>,
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionResponse {
    success: bool,
    id: String,
    message: String,
    checkout_url: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/upload", get(presign_solution_upload))
        .route("/", post(submit_solution))
}

#[tracing::instrument(name = "GET /solution/upload", skip(state))]
async fn presign_solution_upload(
    State(state): State<AppState>,
    Query(params): Query<UploadParams>,
) -> Result<Json<SolutionUploadResponse>, ApiError> {
    let id = create_id();
    let ext = sanitize_ext(params.extension.as_deref()).unwrap_or_else(|| "bin".to_string());

    let now_utc = Utc::now();
    let date_prefix = now_utc.format("%Y/%m/%d").to_string();
    let file_name = format!("{id}.{ext}");
    let key = format!("solution-requests/{date_prefix}/{file_name}");

    let content_type: Mime = params
        .content_type
        .as_deref()
        .and_then(|s| s.parse::<Mime>().ok())
        .or_else(|| mime_guess::from_ext(&ext).first())
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);

    let master = state.master_credentials().await?;
    let store = master.to_store(false).await?;
    let path = FLPath::from(key.clone());

    let (download_url, upload_url) = try_join!(
        store.sign("GET", &path, Duration::from_secs(DOWNLOAD_TTL_SECS)),
        store.sign("PUT", &path, Duration::from_secs(UPLOAD_TTL_SECS)),
    )?;

    let download_expires_at =
        (now_utc + ChronoDuration::seconds(DOWNLOAD_TTL_SECS as i64)).to_rfc3339();
    let upload_expires_at =
        (now_utc + ChronoDuration::seconds(UPLOAD_TTL_SECS as i64)).to_rfc3339();

    let response = SolutionUploadResponse {
        key,
        content_type: content_type.to_string(),
        upload_url: upload_url.to_string(),
        upload_expires_at,
        download_url: download_url.to_string(),
        download_expires_at,
        size_limit_bytes: SIZE_LIMIT_BYTES,
    };

    Ok(Json(response))
}

#[tracing::instrument(name = "POST /solution", skip(state, submission))]
async fn submit_solution(
    State(state): State<AppState>,
    Json(submission): Json<SolutionSubmission>,
) -> Result<Json<SubmissionResponse>, ApiError> {
    use crate::entity::{
        sea_orm_active_enums::{SolutionPricingTier, SolutionStatus},
        solution_request,
    };

    if submission.name.trim().is_empty() {
        return Err(ApiError::BadRequest("Name is required".to_string()));
    }
    if submission.email.trim().is_empty() || !submission.email.contains('@') {
        return Err(ApiError::BadRequest("Valid email is required".to_string()));
    }
    if submission.company.trim().is_empty() {
        return Err(ApiError::BadRequest("Company is required".to_string()));
    }
    if submission.description.len() < 50 {
        return Err(ApiError::BadRequest(
            "Description must be at least 50 characters".to_string(),
        ));
    }
    if submission.example_input.len() < 20 {
        return Err(ApiError::BadRequest(
            "Example input must be at least 20 characters".to_string(),
        ));
    }
    if submission.expected_output.len() < 20 {
        return Err(ApiError::BadRequest(
            "Expected output must be at least 20 characters".to_string(),
        ));
    }

    let (tier_name, total_cents, pricing_tier_enum) = match submission.pricing_tier.as_str() {
        "standard" => (
            "24 Hour Solution - Standard",
            STANDARD_TOTAL_CENTS,
            SolutionPricingTier::Standard,
        ),
        "appstore" => (
            "24 Hour Solution - App Store",
            APPSTORE_TOTAL_CENTS,
            SolutionPricingTier::Appstore,
        ),
        _ => {
            return Err(ApiError::BadRequest(
                "Invalid pricing tier. Must be 'standard' or 'appstore'".to_string(),
            ));
        }
    };

    let deposit_cents = if submission.pay_deposit {
        DEPOSIT_AMOUNT_CENTS
    } else {
        0
    };
    let remainder_cents = total_cents - deposit_cents;

    let submission_id = create_id();
    let now_utc = Utc::now();
    let date_prefix = now_utc.format("%Y/%m/%d").to_string();

    let submission_data = serde_json::to_string_pretty(&submission)?;
    let key = format!("solution-requests/{date_prefix}/submissions/{submission_id}.json");

    let master = state.master_credentials().await?;
    let store = master.to_store(false).await?;
    let path = FLPath::from(key.clone());

    store
        .as_generic()
        .put(&path, submission_data.into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to store submission: {}", e)))?;

    let files_json = serde_json::to_value(&submission.files).ok();

    let new_solution = solution_request::ActiveModel {
        id: Set(submission_id.clone()),
        name: Set(submission.name.clone()),
        email: Set(submission.email.clone()),
        company: Set(submission.company.clone()),
        description: Set(submission.description.clone()),
        application_type: Set(submission.application_type.clone()),
        data_security: Set(submission.data_security.clone()),
        example_input: Set(submission.example_input.clone()),
        expected_output: Set(submission.expected_output.clone()),
        user_count: Set(submission.user_count.clone()),
        user_type: Set(submission.user_type.clone()),
        technical_level: Set(submission.technical_level.clone()),
        timeline: Set(submission.timeline.clone()),
        additional_notes: Set(submission.additional_notes.clone()),
        pricing_tier: Set(pricing_tier_enum),
        paid_deposit: Set(false),
        files: Set(files_json),
        storage_key: Set(Some(key.clone())),
        status: Set(SolutionStatus::PendingPayment),
        stripe_checkout_session_id: Set(None),
        stripe_payment_intent_id: Set(None),
        stripe_setup_intent_id: Set(None),
        total_cents: Set(total_cents),
        deposit_cents: Set(deposit_cents),
        remainder_cents: Set(remainder_cents),
        priority: Set(submission.pay_deposit),
        admin_notes: Set(None),
        assigned_to: Set(None),
        delivered_at: Set(None),
        created_at: Set(now_utc.naive_utc()),
        updated_at: Set(now_utc.naive_utc()),
    };

    new_solution.insert(&state.db).await?;

    tracing::info!(
        submission_id = %submission_id,
        email = %submission.email,
        company = %submission.company,
        pricing_tier = %submission.pricing_tier,
        pay_deposit = %submission.pay_deposit,
        "New 24-hour solution request submitted and stored in database"
    );

    let checkout_url = if let Some(stripe_client) = state.stripe_client.as_ref() {
        let frontend_url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| format!("https://{}", state.platform_config.domain));

        let success_url = format!(
            "{}/24-hour-solution?success=true&session_id={{CHECKOUT_SESSION_ID}}",
            frontend_url
        );
        let cancel_url = format!("{}/24-hour-solution?canceled=true", frontend_url);

        let total_display = format!("€{:.2}", total_cents as f64 / 100.0);

        // Build metadata
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("submission_id".to_string(), submission_id.clone());
        metadata.insert("company".to_string(), submission.company.clone());
        metadata.insert("pricing_tier".to_string(), submission.pricing_tier.clone());
        metadata.insert("storage_key".to_string(), key.clone());
        metadata.insert("total_cents".to_string(), total_cents.to_string());

        let mut params = stripe::CreateCheckoutSession::new();
        params.success_url = Some(&success_url);
        params.cancel_url = Some(&cancel_url);
        params.customer_email = Some(&submission.email);
        params.client_reference_id = Some(&submission_id);

        if submission.pay_deposit {
            // Payment mode: Charge €500 deposit
            params.mode = Some(stripe::CheckoutSessionMode::Payment);

            let remainder_cents = total_cents - DEPOSIT_AMOUNT_CENTS;
            let remainder_display = format!("€{:.2}", remainder_cents as f64 / 100.0);

            let line_item = stripe::CreateCheckoutSessionLineItems {
                price_data: Some(stripe::CreateCheckoutSessionLineItemsPriceData {
                    currency: stripe::Currency::EUR,
                    product_data: Some(
                        stripe::CreateCheckoutSessionLineItemsPriceDataProductData {
                            name: format!("{} (Priority Deposit)", tier_name),
                            description: Some(format!(
                                "Priority deposit for {} | Total: {} | Remaining {} invoiced after delivery",
                                submission.company, total_display, remainder_display
                            )),
                            ..Default::default()
                        },
                    ),
                    unit_amount: Some(DEPOSIT_AMOUNT_CENTS),
                    ..Default::default()
                }),
                quantity: Some(1),
                ..Default::default()
            };
            params.line_items = Some(vec![line_item]);

            metadata.insert(
                "deposit_cents".to_string(),
                DEPOSIT_AMOUNT_CENTS.to_string(),
            );
            metadata.insert("remainder_cents".to_string(), remainder_cents.to_string());
            metadata.insert("priority".to_string(), "true".to_string());
        } else {
            // Setup mode: No charge, just collect customer info for tracking
            params.mode = Some(stripe::CheckoutSessionMode::Setup);

            metadata.insert("deposit_cents".to_string(), "0".to_string());
            metadata.insert("remainder_cents".to_string(), total_cents.to_string());
            metadata.insert("priority".to_string(), "false".to_string());
        }

        params.metadata = Some(metadata);

        match stripe::CheckoutSession::create(stripe_client, params).await {
            Ok(session) => session.url,
            Err(e) => {
                tracing::error!(error = %e, "Failed to create Stripe checkout session");
                None
            }
        }
    } else {
        tracing::warn!("Stripe client not configured, skipping checkout session creation");
        None
    };

    Ok(Json(SubmissionResponse {
        success: true,
        id: submission_id,
        message: if checkout_url.is_some() {
            if submission.pay_deposit {
                "Redirecting to payment...".to_string()
            } else {
                "Redirecting to confirm your request...".to_string()
            }
        } else {
            "Your request has been submitted successfully. We'll review and get back to you within 48 hours.".to_string()
        },
        checkout_url,
    }))
}

fn sanitize_ext(input: Option<&str>) -> Option<String> {
    let mut s = input?.trim().trim_start_matches('.').to_ascii_lowercase();
    if s.is_empty() || s.len() > 16 || !s.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(std::mem::take(&mut s))
}
