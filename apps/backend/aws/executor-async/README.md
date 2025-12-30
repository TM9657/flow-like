# AWS Runtime Async (SQS Consumer)

AWS Lambda function that processes execution requests from SQS queues.
This is the consumer side of the SQS execution backend.

## Architecture

```
API (dispatch) -> SQS Queue -> This Lambda -> Execute Flow -> Callback to API
```

1. API dispatches execution requests to SQS using the `sqs` backend
2. This Lambda is triggered by SQS events (batch processing)
3. Each message is processed and the flow is executed
4. Progress and events are sent back to the API via callback URLs

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `EXECUTOR_BATCH_INTERVAL_MS` | Event batching interval | 1000 |
| `EXECUTOR_MAX_BATCH_SIZE` | Max events per batch | 100 |
| `EXECUTOR_CALLBACK_TIMEOUT_MS` | Callback request timeout | 5000 |
| `EXECUTOR_CALLBACK_RETRIES` | Callback retry count | 3 |
| `EXECUTOR_TIMEOUT_SECS` | Execution timeout | 3600 |

## SQS Configuration

Configure the SQS trigger with:
- **Batch size**: 1-10 messages (start with 1 for debugging)
- **Batch window**: 0-300 seconds
- **Report batch item failures**: Enabled (for partial batch failure handling)

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)

## Building

To build the project for production, run `cargo lambda build --release`. Remove the `--release` flag to build for development.

Read more about building your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/build.html).

## Testing

You can run regular Rust unit tests with `cargo test`.

If you want to run integration tests locally, you can use the `cargo lambda watch` and `cargo lambda invoke` commands to do it.

First, run `cargo lambda watch` to start a local server. When you make changes to the code, the server will automatically restart.

Second, you'll need a way to pass the event data to the lambda function.

You can use the existent [event payloads](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) in the Rust Runtime repository if your lambda function is using one of the supported event types.

You can use those examples directly with the `--data-example` flag, where the value is the name of the file in the [lambda-events](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) repository without the `example_` prefix and the `.json` extension.

```bash
cargo lambda invoke --data-example apigw-request
```

For generic events, where you define the event data structure, you can create a JSON file with the data you want to test with. For example:

```json
{
    "command": "test"
}
```

Then, run `cargo lambda invoke --data-file ./data.json` to invoke the function with the data in `data.json`.

For HTTP events, you can also call the function directly with cURL or any other HTTP client. For example:

```bash
curl https://localhost:9000
```

Read more about running the local server in [the Cargo Lambda documentation for the `watch` command](https://www.cargo-lambda.info/commands/watch.html).
Read more about invoking the function in [the Cargo Lambda documentation for the `invoke` command](https://www.cargo-lambda.info/commands/invoke.html).

## Deploying

To deploy the project, run `cargo lambda deploy`. This will create an IAM role and a Lambda function in your AWS account.

Read more about deploying your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/deploy.html).
