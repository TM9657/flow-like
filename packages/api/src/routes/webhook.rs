use crate::{error::ApiError, state::AppState};
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use flow_like_types::anyhow;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use stripe::{Event, EventObject, EventType, Webhook};

fn get_stripe_webhook_secret() -> Option<String> {
    std::env::var("STRIPE_WEBHOOK_SECRET").ok()
}

#[tracing::instrument(name = "POST /webhook/stripe", skip(state, headers, payload))]
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let stripe_client = state
        .stripe_client
        .as_ref()
        .ok_or(anyhow!("Stripe not configured"))?;

    let webhook_secret =
        get_stripe_webhook_secret().ok_or(anyhow!("Webhook secret not configured"))?;

    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(anyhow!("Missing stripe-signature header"))?;

    let payload_str =
        std::str::from_utf8(&payload).map_err(|_| anyhow!("Invalid UTF-8 in payload"))?;

    let event = Webhook::construct_event(payload_str, signature, &webhook_secret)
        .map_err(|e| anyhow!("Failed to verify webhook signature: {}", e))?;

    let event_id = event.id.to_string();
    if is_event_processed(&state, &event_id).await? {
        tracing::info!(event_id = %event_id, "Duplicate event, skipping");
        return Ok(StatusCode::OK);
    }

    match handle_stripe_event(&state, stripe_client, &event).await {
        Ok(_) => {
            mark_event_processed(&state, &event_id, &event.type_.to_string()).await?;
            Ok(StatusCode::OK)
        }
        Err(e) => {
            tracing::error!(event_id = %event_id, "Failed to process webhook");
            Err(e)
        }
    }
}

async fn is_event_processed(state: &AppState, event_id: &str) -> Result<bool, ApiError> {
    use crate::entity::stripe_event;

    let existing = stripe_event::Entity::find_by_id(event_id)
        .one(&state.db)
        .await?;

    Ok(existing.is_some())
}

async fn mark_event_processed(
    state: &AppState,
    event_id: &str,
    event_type: &str,
) -> Result<(), ApiError> {
    use crate::entity::stripe_event;

    let new_event = stripe_event::ActiveModel {
        id: Set(event_id.to_string()),
        event_type: Set(event_type.to_string()),
        processed_at: Set(chrono::Utc::now().naive_utc()),
    };

    new_event.insert(&state.db).await?;
    Ok(())
}

async fn handle_stripe_event(
    state: &AppState,
    _stripe_client: &stripe::Client,
    event: &Event,
) -> Result<(), ApiError> {
    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            if let EventObject::CheckoutSession(session) = &event.data.object {
                handle_checkout_completed(state, session).await?;
            }
        }
        EventType::CheckoutSessionExpired => {
            if let EventObject::CheckoutSession(session) = &event.data.object {
                handle_checkout_expired(state, session).await?;
            }
        }
        EventType::CustomerSubscriptionCreated
        | EventType::CustomerSubscriptionUpdated
        | EventType::CustomerSubscriptionDeleted => {
            if let EventObject::Subscription(subscription) = &event.data.object {
                handle_subscription_change(state, subscription, &event.type_).await?;
            }
        }
        EventType::PaymentIntentSucceeded => {
            if let EventObject::PaymentIntent(intent) = &event.data.object {
                handle_payment_intent_succeeded(state, intent).await?;
            }
        }
        EventType::PaymentIntentPaymentFailed => {
            if let EventObject::PaymentIntent(intent) = &event.data.object {
                handle_payment_intent_failed(state, intent).await?;
            }
        }
        _ => {
            tracing::debug!(event_type = %event.type_, "Unhandled event type");
        }
    }

    Ok(())
}

async fn handle_checkout_completed(
    state: &AppState,
    session: &stripe::CheckoutSession,
) -> Result<(), ApiError> {
    use crate::entity::solution_request;

    let session_id = session.id.to_string();

    tracing::info!(
        session_id = %session_id,
        client_reference_id = ?session.client_reference_id,
        mode = ?session.mode,
        payment_status = ?session.payment_status,
        "Processing checkout.session.completed for solution"
    );

    let submission_id = session
        .client_reference_id
        .as_ref()
        .ok_or(anyhow!("Missing client_reference_id"))?;

    let existing = solution_request::Entity::find_by_id(submission_id.clone())
        .one(&state.db)
        .await?;

    if let Some(solution) = existing {
        let mut active: solution_request::ActiveModel = solution.into();

        active.stripe_checkout_session_id = Set(Some(session_id.clone()));

        if let Some(pi) = &session.payment_intent {
            active.stripe_payment_intent_id = Set(Some(pi.id().to_string()));
        }

        if let Some(si) = &session.setup_intent {
            active.stripe_setup_intent_id = Set(Some(si.id().to_string()));
        }

        // Mark deposit as paid and move to PendingReview (in queue)
        active.paid_deposit = Set(true);
        active.status = Set(crate::entity::sea_orm_active_enums::SolutionStatus::PendingReview);
        active.updated_at = Set(chrono::Utc::now().naive_utc());

        active.update(&state.db).await?;

        tracing::info!(
            submission_id = %submission_id,
            "Solution request updated: paid_deposit=true, status=PENDING_REVIEW"
        );
    } else {
        tracing::warn!(
            submission_id = %submission_id,
            "Solution request not found for checkout session"
        );
    }

    Ok(())
}

async fn handle_checkout_expired(
    state: &AppState,
    session: &stripe::CheckoutSession,
) -> Result<(), ApiError> {
    use crate::entity::solution_request;

    let session_id = session.id.to_string();

    if let Some(submission_id) = &session.client_reference_id {
        let existing = solution_request::Entity::find_by_id(submission_id.clone())
            .one(&state.db)
            .await?;

        if let Some(solution) = existing {
            let mut active: solution_request::ActiveModel = solution.into();
            active.status = Set(crate::entity::sea_orm_active_enums::SolutionStatus::Cancelled);
            active.updated_at = Set(chrono::Utc::now().naive_utc());
            active.update(&state.db).await?;

            tracing::info!(
                submission_id = %submission_id,
                session_id = %session_id,
                "Solution request marked as CANCELLED due to expired checkout"
            );
        }
    }

    Ok(())
}

async fn handle_subscription_change(
    state: &AppState,
    subscription: &stripe::Subscription,
    event_type: &EventType,
) -> Result<(), ApiError> {
    use crate::entity::{sea_orm_active_enums::UserTier, user};

    let customer_id = match &subscription.customer {
        stripe::Expandable::Id(id) => id.to_string(),
        stripe::Expandable::Object(c) => c.id.to_string(),
    };

    tracing::info!(
        subscription_id = %subscription.id,
        customer_id = %customer_id,
        event_type = %event_type,
        status = ?subscription.status,
        "Processing subscription change"
    );

    let user_result = user::Entity::find()
        .filter(user::Column::StripeId.eq(&customer_id))
        .one(&state.db)
        .await?;

    if let Some(user_model) = user_result {
        let new_tier = match subscription.status {
            stripe::SubscriptionStatus::Active | stripe::SubscriptionStatus::Trialing => {
                determine_tier_from_subscription(state, subscription)
            }
            stripe::SubscriptionStatus::Canceled
            | stripe::SubscriptionStatus::Unpaid
            | stripe::SubscriptionStatus::IncompleteExpired => UserTier::Free,
            _ => return Ok(()),
        };

        let mut active: user::ActiveModel = user_model.into();
        active.tier = Set(new_tier.clone());
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active.update(&state.db).await?;

        tracing::info!(
            customer_id = %customer_id,
            new_tier = ?new_tier,
            "User tier updated based on subscription"
        );
    } else {
        tracing::warn!(
            customer_id = %customer_id,
            "No user found for Stripe customer"
        );
    }

    Ok(())
}

fn determine_tier_from_subscription(
    state: &AppState,
    subscription: &stripe::Subscription,
) -> crate::entity::sea_orm_active_enums::UserTier {
    use crate::entity::sea_orm_active_enums::UserTier;

    for item in &subscription.items.data {
        if let Some(price) = &item.price {
            if let Some(product) = &price.product {
                let product_id = match product {
                    stripe::Expandable::Id(id) => id.to_string(),
                    stripe::Expandable::Object(p) => p.id.to_string(),
                };

                // Check product_id against hub config tiers
                for (tier_name, tier_config) in &state.platform_config.tiers {
                    if let Some(config_product_id) = &tier_config.product_id
                        && config_product_id == &product_id
                    {
                        return match tier_name.to_uppercase().as_str() {
                            "ENTERPRISE" => UserTier::Enterprise,
                            "PRO" => UserTier::Pro,
                            "PREMIUM" => UserTier::Premium,
                            _ => UserTier::Free,
                        };
                    }
                }

                // Fallback: check if product_id contains tier name
                let product_lower = product_id.to_lowercase();
                if product_lower.contains("enterprise") {
                    return UserTier::Enterprise;
                } else if product_lower.contains("pro") {
                    return UserTier::Pro;
                } else if product_lower.contains("premium") {
                    return UserTier::Premium;
                }
            }

            // Also check price metadata for tier info
            if let Some(metadata) = &price.metadata
                && let Some(tier) = metadata.get("tier")
            {
                match tier.to_uppercase().as_str() {
                    "ENTERPRISE" => return UserTier::Enterprise,
                    "PRO" => return UserTier::Pro,
                    "PREMIUM" => return UserTier::Premium,
                    _ => {}
                }
            }
        }
    }

    // Default to Free if no matching tier found
    UserTier::Free
}

async fn handle_payment_intent_succeeded(
    state: &AppState,
    intent: &stripe::PaymentIntent,
) -> Result<(), ApiError> {
    use crate::entity::{solution_request, transaction};

    let intent_id = intent.id.to_string();

    tracing::info!(
        payment_intent_id = %intent_id,
        amount = ?intent.amount,
        "Processing payment_intent.succeeded"
    );

    let solution = solution_request::Entity::find()
        .filter(solution_request::Column::StripePaymentIntentId.eq(&intent_id))
        .one(&state.db)
        .await?;

    if let Some(sol) = solution {
        let mut active: solution_request::ActiveModel = sol.into();
        active.paid_deposit = Set(true);
        active.priority = Set(true);
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active.update(&state.db).await?;

        tracing::info!(
            payment_intent_id = %intent_id,
            "Solution request marked as paid with priority"
        );
    }

    if let Some(customer) = &intent.customer {
        let customer_id = match customer {
            stripe::Expandable::Id(id) => id.to_string(),
            stripe::Expandable::Object(c) => c.id.to_string(),
        };

        let user = crate::entity::user::Entity::find()
            .filter(crate::entity::user::Column::StripeId.eq(&customer_id))
            .one(&state.db)
            .await?;

        if let Some(u) = user {
            let existing_tx = transaction::Entity::find()
                .filter(transaction::Column::StripeId.eq(&intent_id))
                .one(&state.db)
                .await?;

            if existing_tx.is_none() {
                let new_tx = transaction::ActiveModel {
                    id: Set(flow_like_types::create_id()),
                    user_id: Set(Some(u.id.clone())),
                    stripe_id: Set(intent_id.clone()),
                    created_at: Set(chrono::Utc::now().naive_utc()),
                    updated_at: Set(chrono::Utc::now().naive_utc()),
                };
                new_tx.insert(&state.db).await?;

                tracing::info!(
                    user_id = %u.id,
                    payment_intent_id = %intent_id,
                    "Transaction recorded"
                );
            }
        }
    }

    Ok(())
}

async fn handle_payment_intent_failed(
    state: &AppState,
    intent: &stripe::PaymentIntent,
) -> Result<(), ApiError> {
    use crate::entity::solution_request;

    let intent_id = intent.id.to_string();

    tracing::warn!(
        payment_intent_id = %intent_id,
        "Processing payment_intent.payment_failed"
    );

    let solution = solution_request::Entity::find()
        .filter(solution_request::Column::StripePaymentIntentId.eq(&intent_id))
        .one(&state.db)
        .await?;

    if let Some(sol) = solution {
        let mut active: solution_request::ActiveModel = sol.into();
        active.admin_notes = Set(Some(format!(
            "Payment failed at {}",
            chrono::Utc::now().to_rfc3339()
        )));
        active.updated_at = Set(chrono::Utc::now().naive_utc());
        active.update(&state.db).await?;
    }

    Ok(())
}
