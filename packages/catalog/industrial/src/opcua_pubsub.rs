use crate::runtime::Session;
use flow_like_industrial::opcua_pubsub::{OpcUaPubSubConfig, PubSubMessage, PubSubValue};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PublishRequest {
    pub session: Session,
    /// Values follow the field order declared when opening the publisher.
    pub values: Vec<PubSubValue>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct SubscribeRequest {
    pub session: Session,
    /// Zero continues until the session or workflow is stopped.
    #[serde(default)]
    pub max_messages: u64,
}

#[crate::register_node]
#[derive(Default)]
pub struct OpcUaPubSubConnectNode;
crate::operation!(
    OpcUaPubSubConnectNode,
    "industrial_opcua_pubsub_connect",
    "Open OPC UA PubSub",
    "Open a local UADP UDP publisher or subscriber with configured scalar field metadata. Supports plaintext or PubSub-Aes256-CTR group keys supplied through the security configuration. Requires a local executor.",
    "opcua_pubsub",
    "connect",
    "Industrial/OPC UA PubSub",
    OpcUaPubSubConfig,
    Session,
    connect,
    sensitive
);

#[crate::register_node]
#[derive(Default)]
pub struct OpcUaPubSubPublishNode;
crate::operation!(
    OpcUaPubSubPublishNode,
    "industrial_opcua_pubsub_publish",
    "Publish OPC UA DataSet",
    "Send one typed UADP key frame and return its sequence number. UDP delivery is not acknowledged.",
    "opcua_pubsub",
    "publish",
    "Industrial/OPC UA PubSub",
    PublishRequest,
    u16,
    publish
);

#[crate::register_node]
#[derive(Default)]
pub struct OpcUaPubSubReceiveNode;
crate::operation!(
    OpcUaPubSubReceiveNode,
    "industrial_opcua_pubsub_receive",
    "Receive OPC UA DataSet",
    "Receive one matching UADP DataSet within the configured timeout and validate its field types and metadata version",
    "opcua_pubsub",
    "receive",
    "Industrial/OPC UA PubSub",
    Session,
    PubSubMessage,
    receive
);

#[crate::register_node]
#[derive(Default)]
pub struct OpcUaPubSubSubscribeNode;
crate::listener!(
    OpcUaPubSubSubscribeNode,
    "industrial_opcua_pubsub_subscribe",
    "Subscribe OPC UA PubSub",
    "Call the referenced handler for matching scalar UADP key frames. Timeout or malformed matching data stops the listener.",
    "opcua_pubsub",
    "subscribe",
    "Industrial/OPC UA PubSub",
    SubscribeRequest,
    u64,
    subscribe
);

#[crate::register_node]
#[derive(Default)]
pub struct OpcUaPubSubDisconnectNode;
crate::operation!(
    OpcUaPubSubDisconnectNode,
    "industrial_opcua_pubsub_disconnect",
    "Close OPC UA PubSub",
    "Cancel pending receives and handlers, leave the multicast group, and close the UDP socket",
    "opcua_pubsub",
    "disconnect",
    "Industrial/OPC UA PubSub",
    Session,
    bool,
    disconnect
);

#[cfg(feature = "execute")]
use {
    crate::runtime::{self, Handler},
    flow_like::flow::execution::{ExecutionEnvironment, context::ExecutionContext},
    flow_like_industrial::opcua_pubsub::{OpcUaPubSubClient, PubSubMode},
    flow_like_types::{Result, anyhow},
    serde_json::json,
};

#[cfg(feature = "execute")]
async fn connect(context: &mut ExecutionContext, config: OpcUaPubSubConfig) -> Result<Session> {
    if context.execution_environment() == ExecutionEnvironment::Server {
        return Err(anyhow!(
            "OPC UA PubSub UDP sockets require a local executor"
        ));
    }
    let client = tokio::select! {
        _ = runtime::wait_for_cancel(context.get_cancellation_token()) => return Err(anyhow!("Execution was cancelled")),
        result = async {
            if config.security.is_some() {
                let directory = if config.mode == PubSubMode::Publisher {
                    runtime::local_app_storage_directory(context, "opcua-pubsub-security")?
                } else {
                    let node_id = context.node.node_id();
                    if node_id.is_empty() || node_id.len() > 80 {
                        return Err(anyhow!("Secure PubSub reader requires a node ID of 1..80 bytes"));
                    }
                    let id: String = node_id.as_bytes().iter().map(|b| format!("{b:02x}")).collect();
                    runtime::local_storage_directory(context, &format!("opcua-pubsub-reader-{id}"))?
                };
                Ok::<_, flow_like_types::Error>(OpcUaPubSubClient::connect_with_security_store(config, &directory).await?)
            } else {
                Ok(OpcUaPubSubClient::connect(config).await?)
            }
        } => result?,
    };
    let session = runtime::store(context, "opcua_pubsub", client).await;
    let client = runtime::get::<OpcUaPubSubClient>(context, &session, "opcua_pubsub").await?;
    let token = runtime::cancellation(context, &session).await?;
    tokio::spawn(async move {
        token.cancelled().await;
        if let Err(error) = client.close().await {
            tracing::warn!("OPC UA PubSub shutdown failed: {error}");
        }
    });
    Ok(session)
}

#[cfg(feature = "execute")]
async fn publish(context: &mut ExecutionContext, input: PublishRequest) -> Result<u16> {
    let client = runtime::get::<OpcUaPubSubClient>(context, &input.session, "opcua_pubsub").await?;
    let token = runtime::cancellation(context, &input.session).await?;
    tokio::select! {
        _ = token.cancelled() => Err(anyhow!("OPC UA PubSub session was closed")),
        _ = runtime::wait_for_cancel(context.get_cancellation_token()) => Err(anyhow!("Execution was cancelled")),
        result = client.publish(&input.values) => Ok(result?),
    }
}

#[cfg(feature = "execute")]
async fn receive(context: &mut ExecutionContext, session: Session) -> Result<PubSubMessage> {
    let client = runtime::get::<OpcUaPubSubClient>(context, &session, "opcua_pubsub").await?;
    let token = runtime::cancellation(context, &session).await?;
    tokio::select! {
        _ = token.cancelled() => Err(anyhow!("OPC UA PubSub session was closed")),
        _ = runtime::wait_for_cancel(context.get_cancellation_token()) => Err(anyhow!("Execution was cancelled")),
        result = client.receive() => Ok(result?),
    }
}

#[cfg(feature = "execute")]
async fn subscribe(context: &mut ExecutionContext, input: SubscribeRequest) -> Result<u64> {
    let client = runtime::get::<OpcUaPubSubClient>(context, &input.session, "opcua_pubsub").await?;
    let token = runtime::cancellation(context, &input.session).await?;
    let mut handler = Handler::new(context).await?;
    let mut count = 0;
    loop {
        let message = tokio::select! {
            _ = token.cancelled() => break,
            _ = runtime::wait_for_cancel(context.get_cancellation_token()) => break,
            result = client.receive() => result?,
        };
        tokio::select! {
            _ = token.cancelled() => break,
            _ = runtime::wait_for_cancel(context.get_cancellation_token()) => break,
            result = handler.dispatch(json!({"session": input.session, "message": message})) => result?,
        }
        count += 1;
        if input.max_messages > 0 && count >= input.max_messages {
            break;
        }
    }
    Ok(count)
}

#[cfg(feature = "execute")]
async fn disconnect(context: &mut ExecutionContext, session: Session) -> Result<bool> {
    let client = runtime::get::<OpcUaPubSubClient>(context, &session, "opcua_pubsub").await?;
    runtime::remove(context, &session).await?;
    client.close().await?;
    Ok(true)
}

#[cfg(all(test, feature = "execute"))]
mod tests {
    use super::*;
    use flow_like::flow::node::{Node, NodeLogic};
    use flow_like_industrial::opcua_pubsub::{
        PubSubDataType, PubSubFieldMetaData, PubSubMode, PubSubWriter, PublisherId,
    };
    use std::{sync::Arc, time::Duration};
    use tokio_util::sync::CancellationToken;

    struct WaitingHandler {
        started: CancellationToken,
        dropped: CancellationToken,
    }

    #[flow_like_types::async_trait]
    impl NodeLogic for WaitingHandler {
        fn get_node(&self) -> Node {
            Node::new("waiting", "Waiting", "Wait for cancellation", "Test")
        }
        async fn run(&self, _: &mut ExecutionContext) -> Result<()> {
            let _dropped = self.dropped.clone().drop_guard();
            self.started.cancel();
            std::future::pending().await
        }
    }

    #[tokio::test]
    async fn disconnect_cancels_an_active_handler_without_cancelling_the_workflow() {
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let mut config = OpcUaPubSubConfig {
            address: format!("opc.udp://127.0.0.1:{port}"),
            interface: "127.0.0.1".into(),
            mode: PubSubMode::Subscriber,
            writer: PubSubWriter {
                publisher_id: PublisherId::UInt16(7),
                writer_group_id: 1,
                data_set_writer_id: 1,
            },
            fields: vec![PubSubFieldMetaData {
                name: "temperature".into(),
                data_type: PubSubDataType::Double,
            }],
            configuration_version: None,
            timeout_ms: 10_000,
            max_datagram_bytes: 1400,
            security: None,
        };
        let started = CancellationToken::new();
        let dropped = CancellationToken::new();
        let workflow = CancellationToken::new();
        let mut context = crate::tests::context_with_logic(Arc::new(WaitingHandler {
            started: started.clone(),
            dropped: dropped.clone(),
        }))
        .await;
        context.set_cancellation_token(workflow.clone());
        let client = OpcUaPubSubClient::connect(config.clone()).await.unwrap();
        let session = runtime::store(&context, "opcua_pubsub", client).await;
        let mut subscribing_context = context.create_sub_context(&context.node).await;
        let request = SubscribeRequest {
            session: session.clone(),
            max_messages: 0,
        };
        let subscribing =
            tokio::spawn(async move { subscribe(&mut subscribing_context, request).await });
        config.mode = PubSubMode::Publisher;
        let publisher = OpcUaPubSubClient::connect(config).await.unwrap();
        publisher
            .publish(&[PubSubValue::Double(23.5)])
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), started.cancelled())
            .await
            .unwrap();
        disconnect(&mut context, session).await.unwrap();
        let delivered = tokio::time::timeout(Duration::from_secs(2), subscribing)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            delivered, 0,
            "An interrupted handler is not a completed delivery"
        );
        assert!(dropped.is_cancelled());
        assert!(!workflow.is_cancelled());
        publisher.close().await.unwrap();
    }
}
