#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use aws_lambda_events::sqs::{BatchItemFailure, SqsBatchResponse, SqsEvent};
use flow_like_types::tokio;
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};
mod execution;

#[flow_like_types::tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(sqs_function_handler)).await
}

pub async fn sqs_function_handler(event: LambdaEvent<SqsEvent>) -> Result<SqsBatchResponse, Error> {
    let mut batch_item_failures = Vec::new();

    for record in event.payload.records.iter() {
        let body = record.body.as_deref().unwrap_or_default();

        // ... Process the message, if it fails, add to batch_item_failures
        match execution::execute(body).await {
            Ok(_) => {
                continue;
            }
            Err(e) => {
                tracing::error!("Failed to process message: {}", e);
                batch_item_failures.push(BatchItemFailure {
                    item_identifier: record.message_id.as_ref().unwrap().clone(),
                });
            }
        }
    }

    Ok(SqsBatchResponse {
        batch_item_failures,
    })
}
