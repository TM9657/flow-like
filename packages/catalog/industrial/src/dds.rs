use crate::runtime::Session;
use flow_like_industrial::dds::{DdsConfig, DdsSample};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct DdsPublishRequest {
    pub session: Session,
    pub data: serde_json::Value,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct DdsSubscribeRequest {
    pub session: Session,
    /// Zero keeps listening until cancellation or disconnect.
    #[serde(default)]
    pub max_samples: u64,
}

#[crate::register_node]
#[derive(Default)]
pub struct DdsConnectNode;
crate::operation!(
    DdsConnectNode,
    "industrial_dds_connect",
    "Connect DDS",
    "Open a DDS publisher or subscriber for a final-struct topic on a local executor. Configure the exact IDL type name and ordered fields.",
    "dds",
    "connect",
    "Industrial/DDS",
    DdsConfig,
    Session,
    connect,
    sensitive
);

#[crate::register_node]
#[derive(Default)]
pub struct DdsWaitForMatchNode;
crate::operation!(
    DdsWaitForMatchNode,
    "industrial_dds_wait_for_match",
    "Wait for DDS Peer",
    "Wait for a compatible DDS endpoint in the configured domain and topic, up to the session timeout",
    "dds",
    "wait_for_match",
    "Industrial/DDS",
    Session,
    u32,
    wait_for_match
);

#[crate::register_node]
#[derive(Default)]
pub struct DdsPublishNode;
crate::operation!(
    DdsPublishNode,
    "industrial_dds_publish",
    "Publish DDS",
    "Publish a JSON object matching the session's DDS schema. Requires a discovered reader; reliable publication waits for acknowledgments.",
    "dds",
    "publish",
    "Industrial/DDS",
    DdsPublishRequest,
    bool,
    publish
);

#[crate::register_node]
#[derive(Default)]
pub struct DdsReceiveNode;
crate::operation!(
    DdsReceiveNode,
    "industrial_dds_receive",
    "Receive DDS Sample",
    "Take one sample or instance lifecycle notification from a subscriber session, up to the configured timeout",
    "dds",
    "receive",
    "Industrial/DDS",
    Session,
    DdsSample,
    receive
);

#[crate::register_node]
#[derive(Default)]
pub struct DdsSubscribeNode;
crate::listener!(
    DdsSubscribeNode,
    "industrial_dds_subscribe",
    "Subscribe DDS",
    "Call the handler for each typed DDS sample or instance lifecycle notification until cancellation or the sample limit",
    "dds",
    "subscribe",
    "Industrial/DDS",
    DdsSubscribeRequest,
    u64,
    subscribe
);

#[crate::register_node]
#[derive(Default)]
pub struct DdsDisconnectNode;
crate::operation!(
    DdsDisconnectNode,
    "industrial_dds_disconnect",
    "Disconnect DDS",
    "Stop pending DDS operations and remove the topic endpoint and domain participant",
    "dds",
    "disconnect",
    "Industrial/DDS",
    Session,
    bool,
    disconnect
);

#[cfg(feature = "execute")]
use {
    crate::runtime::{self, Handler},
    flow_like::flow::execution::{ExecutionEnvironment, context::ExecutionContext},
    flow_like_industrial::dds::DdsClient,
    flow_like_types::{Result, anyhow},
    serde_json::json,
};

#[cfg(feature = "execute")]
async fn connect(context: &mut ExecutionContext, config: DdsConfig) -> Result<Session> {
    if context.execution_environment() == ExecutionEnvironment::Server {
        return Err(anyhow!(
            "DDS discovery requires a local executor; shared server egress cannot validate discovered peers"
        ));
    }
    let client = tokio::select! {
        biased;
        _ = runtime::wait_for_cancel(context.get_cancellation_token()) => return Err(anyhow!("DDS connect cancelled")),
        result = DdsClient::connect(config) => result?,
    };
    let session = runtime::store(context, "dds", client).await;
    let client = runtime::get::<DdsClient>(context, &session, "dds").await?;
    let stop = runtime::cancellation(context, &session).await?;
    tokio::spawn(async move {
        stop.cancelled().await;
        let _ = client.close().await;
    });
    Ok(session)
}

#[cfg(feature = "execute")]
async fn wait_for_match(context: &mut ExecutionContext, session: Session) -> Result<u32> {
    let client = runtime::get::<DdsClient>(context, &session, "dds").await?;
    let stop = runtime::cancellation(context, &session).await?;
    tokio::select! {
        biased;
        _ = stop.cancelled() => Err(anyhow!("DDS session closed")),
        _ = runtime::wait_for_cancel(context.get_cancellation_token()) => Err(anyhow!("DDS wait cancelled")),
        result = client.wait_for_match() => Ok(result?),
    }
}

#[cfg(feature = "execute")]
async fn publish(context: &mut ExecutionContext, input: DdsPublishRequest) -> Result<bool> {
    let client = runtime::get::<DdsClient>(context, &input.session, "dds").await?;
    let stop = runtime::cancellation(context, &input.session).await?;
    tokio::select! {
        biased;
        _ = stop.cancelled() => return Err(anyhow!("DDS session closed")),
        _ = runtime::wait_for_cancel(context.get_cancellation_token()) => return Err(anyhow!("DDS publication cancelled")),
        result = client.publish(input.data) => result?,
    }
    Ok(true)
}

#[cfg(feature = "execute")]
async fn receive(context: &mut ExecutionContext, session: Session) -> Result<DdsSample> {
    let client = runtime::get::<DdsClient>(context, &session, "dds").await?;
    let stop = runtime::cancellation(context, &session).await?;
    tokio::select! {
        biased;
        _ = stop.cancelled() => Err(anyhow!("DDS session closed")),
        _ = runtime::wait_for_cancel(context.get_cancellation_token()) => Err(anyhow!("DDS receive cancelled")),
        result = client.receive() => Ok(result?),
    }
}

#[cfg(feature = "execute")]
async fn subscribe(context: &mut ExecutionContext, input: DdsSubscribeRequest) -> Result<u64> {
    let mut handler = Handler::new(context).await?;
    let client = runtime::get::<DdsClient>(context, &input.session, "dds").await?;
    let stop = runtime::cancellation(context, &input.session).await?;
    let mut count = 0;
    loop {
        let sample = tokio::select! {
            biased;
            _ = stop.cancelled() => break,
            _ = runtime::wait_for_cancel(context.get_cancellation_token()) => break,
            result = client.receive() => match result {
                Ok(sample) => sample,
                Err(flow_like_industrial::Error::Timeout) => continue,
                Err(error) => return Err(error.into()),
            },
        };
        tokio::select! {
            biased;
            _ = stop.cancelled() => break,
            _ = runtime::wait_for_cancel(context.get_cancellation_token()) => break,
            result = handler.dispatch(json!(sample)) => result?,
        }
        count += 1;
        if input.max_samples > 0 && count >= input.max_samples {
            break;
        }
    }
    Ok(count)
}

#[cfg(feature = "execute")]
async fn disconnect(context: &mut ExecutionContext, session: Session) -> Result<bool> {
    let client = runtime::get::<DdsClient>(context, &session, "dds").await?;
    runtime::remove(context, &session).await?;
    client.close().await?;
    Ok(true)
}

#[cfg(all(test, feature = "execute"))]
mod tests {
    use super::*;
    use flow_like::flow::node::{Node, NodeLogic};
    use flow_like_industrial::dds::DdsRole;
    use std::{sync::Arc, time::Duration};
    use tokio_util::sync::CancellationToken;

    struct HandlerProbe {
        started: CancellationToken,
        dropped: CancellationToken,
        fail: bool,
    }

    #[flow_like_types::async_trait]
    impl NodeLogic for HandlerProbe {
        fn get_node(&self) -> Node {
            Node::new(
                "dds_probe",
                "DDS probe",
                "Observe DDS handler completion",
                "Test",
            )
        }

        async fn run(&self, _: &mut ExecutionContext) -> Result<()> {
            let _dropped = self.dropped.clone().drop_guard();
            self.started.cancel();
            if self.fail {
                Err(anyhow!("DDS test handler failed"))
            } else {
                std::future::pending().await
            }
        }
    }

    async fn handler_case(fail: bool, domain_id: i32) {
        let started = CancellationToken::new();
        let dropped = CancellationToken::new();
        let workflow = CancellationToken::new();
        let mut context = crate::tests::context_with_logic(Arc::new(HandlerProbe {
            started: started.clone(),
            dropped: dropped.clone(),
            fail,
        }))
        .await;
        context.set_cancellation_token(workflow.clone());
        let config = DdsConfig {
            domain_id,
            topic: format!("FlowLikeHandler{domain_id}"),
            role: DdsRole::Subscriber,
            ..Default::default()
        };
        let session = connect(&mut context, config.clone()).await.unwrap();
        let publisher = DdsClient::connect(DdsConfig {
            role: DdsRole::Publisher,
            ..config
        })
        .await
        .unwrap();
        let mut listener_context = context.create_sub_context(&context.node).await;
        let request = DdsSubscribeRequest {
            session: session.clone(),
            max_samples: 0,
        };
        let listening =
            tokio::spawn(async move { subscribe(&mut listener_context, request).await });
        publisher.publish(json!({"value": 23.5})).await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), started.cancelled())
            .await
            .expect("DDS sample must reach its workflow handler");
        if !fail {
            disconnect(&mut context, session.clone()).await.unwrap();
        }
        let result = tokio::time::timeout(Duration::from_secs(2), listening)
            .await
            .expect("DDS listener must stop promptly")
            .unwrap();
        if fail {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("DDS test handler failed")
            );
            disconnect(&mut context, session).await.unwrap();
        } else {
            assert_eq!(
                result.unwrap(),
                0,
                "An interrupted handler is not a completed sample"
            );
        }
        assert!(dropped.is_cancelled(), "The handler future must be dropped");
        assert!(
            !workflow.is_cancelled(),
            "Disconnect must not cancel the workflow"
        );
        publisher.close().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn disconnect_cancels_the_active_handler_without_cancelling_the_workflow() {
        handler_case(false, 180).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn handler_failure_is_reported_by_the_listener() {
        handler_case(true, 181).await;
    }

    #[tokio::test]
    async fn already_cancelled_workflow_does_not_start_discovery() {
        let mut context = crate::tests::context_with_logic(Arc::new(HandlerProbe {
            started: CancellationToken::new(),
            dropped: CancellationToken::new(),
            fail: false,
        }))
        .await;
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        context.set_cancellation_token(cancelled);
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            connect(&mut context, DdsConfig::default()),
        )
        .await
        .unwrap();
        assert!(result.unwrap_err().to_string().contains("cancelled"));
    }
}
