use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Cacheable, anyhow, async_trait, json::json};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio::{self, io::BufStream, net::TcpStream, sync::Mutex};
pub mod send_mail;

use async_smtp::{
    SmtpClient, SmtpTransport,
    authentication::{Credentials, DEFAULT_ENCRYPTED_MECHANISMS},
};

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct SmtpConnection {
    pub id: String,
}

impl SmtpConnection {
    pub fn new(id: String) -> Self {
        SmtpConnection { id }
    }

    pub async fn to_session(
        &self,
        context: &mut ExecutionContext,
    ) -> flow_like_types::Result<SmtpSession> {
        let cache_key = format!("smtp_session_{}", self.id);
        if let Some(session) = context.get_cache(&cache_key).await {
            let session = session
                .as_any()
                .downcast_ref::<SmtpSessionCache>()
                .ok_or_else(|| flow_like_types::anyhow!("Failed to downcast SmtpSessionCache"))?
                .session
                .clone();
            Ok(session)
        } else {
            Err(flow_like_types::anyhow!("SMTP session not found"))
        }
    }

    pub async fn to_session_cache(
        &self,
        context: &mut ExecutionContext,
    ) -> flow_like_types::Result<SmtpSessionCache> {
        let cache_key = format!("smtp_session_{}", self.id);
        if let Some(session) = context.get_cache(&cache_key).await {
            let session = session
                .as_any()
                .downcast_ref::<SmtpSessionCache>()
                .ok_or_else(|| flow_like_types::anyhow!("Failed to downcast SmtpSessionCache"))?
                .clone();
            Ok(session)
        } else {
            Err(flow_like_types::anyhow!("SMTP session not found"))
        }
    }
}

pub type SmtpTransportTls = SmtpTransport<BufStream<async_native_tls::TlsStream<TcpStream>>>;
pub type SmtpSession = Arc<Mutex<SmtpTransportTls>>;

#[derive(Clone)]
pub struct SmtpSessionCache {
    pub session: SmtpSession,
}

impl Cacheable for SmtpSessionCache {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct SmtpConnectNode;

impl SmtpConnectNode {
    pub fn new() -> Self {
        SmtpConnectNode
    }
}

fn tls() -> async_native_tls::TlsConnector {
    async_native_tls::TlsConnector::new()
        .danger_accept_invalid_hostnames(true)
        .danger_accept_invalid_certs(true)
}

#[async_trait]
impl NodeLogic for SmtpConnectNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "email_smtp_connect",
            "SMTP Connect",
            "Connects to an SMTP server and caches the session",
            "Email/SMTP",
        );
        node.add_icon("/flow/icons/mail.svg");

        node.add_input_pin("exec_in", "In", "Execution input", VariableType::Execution);
        node.add_input_pin("host", "Host", "SMTP server hostname", VariableType::String)
            .set_default_value(Some(json!("smtp.example.com")));
        node.add_input_pin("port", "Port", "SMTP server port", VariableType::Integer)
            .set_default_value(Some(json!(587)));
        node.add_input_pin(
            "username",
            "Username",
            "Email account username",
            VariableType::String,
        );
        node.add_input_pin(
            "password",
            "Password",
            "Email account password",
            VariableType::String,
        )
        .set_options(PinOptions::new().set_sensitive(true).build());
        node.add_input_pin(
            "encryption",
            "Encryption",
            "Connection encryption: Tls, StartTls, or Plain",
            VariableType::String,
        )
        .set_default_value(Some(json!("StartTls")))
        .set_options(
            PinOptions::new()
                .set_valid_values(vec!["Tls".into(), "StartTls".into(), "Plain".into()])
                .build(),
        );

        node.add_output_pin(
            "exec_out",
            "Out",
            "Execution output",
            VariableType::Execution,
        );
        node.add_output_pin(
            "connection",
            "Connection",
            "Cached SMTP connection reference",
            VariableType::Struct,
        )
        .set_schema::<SmtpConnection>();

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let host: String = context.evaluate_pin("host").await?;
        let port_f: f64 = context.evaluate_pin("port").await?;
        let port: u16 = port_f as u16;
        let username: String = context.evaluate_pin("username").await?;
        let password: String = context.evaluate_pin("password").await?;
        let encryption: String = context.evaluate_pin("encryption").await?;

        let mut hasher = DefaultHasher::new();
        host.hash(&mut hasher);
        port.hash(&mut hasher);
        username.hash(&mut hasher);
        password.hash(&mut hasher);
        encryption.hash(&mut hasher);
        let id = hasher.finish().to_string();
        let cache_key = format!("smtp_session_{}", id);

        {
            let cache = context.cache.read().await;
            if cache.contains_key(&cache_key) {
                drop(cache);
                context
                    .set_pin_value("connection", json!(SmtpConnection { id: id.clone() }))
                    .await?;
                context.activate_exec_pin("exec_out").await?;
                return Ok(());
            }
        }

        let addr: (&str, u16) = (&host, port);
        context.log_message(
            &format!("-- connecting to {}:{} via {}", addr.0, addr.1, encryption),
            flow_like::flow::execution::LogLevel::Debug,
        );

        let creds = Credentials::new(username.clone(), password.clone());

        let session: SmtpSession = match encryption.as_str() {
            "Tls" => {
                let tcp = TcpStream::connect(addr).await?;
                let tls_stream = tls().connect(&host, tcp).await?;
                let stream = BufStream::new(tls_stream);

                let client = SmtpClient::new(); // expects greeting over TLS
                let transport = SmtpTransport::new(client, stream)
                    .await
                    .map_err(|e| anyhow!("SMTP connect failed: {}", e))?;

                let mut transport = transport;
                transport
                    .try_login(&creds, DEFAULT_ENCRYPTED_MECHANISMS)
                    .await
                    .map_err(|e| anyhow!("SMTP AUTH failed: {}", e))?;

                Arc::new(Mutex::new(transport))
            }
            "StartTls" => {
                let tcp = TcpStream::connect(addr).await?;
                let stream_plain = BufStream::new(tcp);

                let client = SmtpClient::new(); // expect_greeting = true
                let transport_plain = SmtpTransport::new(client, stream_plain)
                    .await
                    .map_err(|e| anyhow!("SMTP connect (pre-STARTTLS) failed: {}", e))?;

                let inner_plain = transport_plain
                    .starttls()
                    .await
                    .map_err(|e| anyhow!("SMTP STARTTLS failed: {}", e))?;

                let tcp_stream = inner_plain.into_inner();
                let tls_stream = tls().connect(&host, tcp_stream).await?;
                let stream_tls = BufStream::new(tls_stream);

                let client_tls = SmtpClient::new().without_greeting();
                let mut transport_tls = SmtpTransport::new(client_tls, stream_tls)
                    .await
                    .map_err(|e| anyhow!("SMTP post-STARTTLS setup failed: {}", e))?;

                transport_tls
                    .try_login(&creds, DEFAULT_ENCRYPTED_MECHANISMS)
                    .await
                    .map_err(|e| anyhow!("SMTP AUTH failed: {}", e))?;

                Arc::new(Mutex::new(transport_tls))
            }
            "Plain" => {
                return Err(flow_like_types::anyhow!(
                    "Plain connection is not supported. Use Tls or StartTls instead."
                ));
            }
            other => {
                return Err(flow_like_types::anyhow!(
                    "Unsupported encryption mode: {} (valid: Tls, StartTls, Plain)",
                    other
                ));
            }
        };

        context.log_message(
            &format!("-- authenticated SMTP as {}", &username),
            flow_like::flow::execution::LogLevel::Debug,
        );

        let cache_obj = SmtpSessionCache { session };
        context
            .cache
            .write()
            .await
            .insert(cache_key, Arc::new(cache_obj));

        context
            .set_pin_value("connection", json!(SmtpConnection { id: id.clone() }))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
