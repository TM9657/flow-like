---
title: Industrial protocols
description: Connect workflows to industrial devices, controllers, and message brokers
sidebar:
  order: 3
---

Industrial nodes connect a workflow to a device or broker, keep the connection for the current run, and pass incoming data to a referenced function. Standard desktop, server, and executor builds include the industrial catalog. An executor still needs network access, device permissions, and any controller driver required by the selected protocol.

## Choose a protocol and role

| Protocol | Available operations | Deployment requirements |
|---|---|---|
| Modbus TCP / RTU | Persistent client, read coils and registers, write coils and holding registers, poll | TCP endpoint or local serial adapter; configure unit ID and register layout |
| OPC UA | Client sessions, browse, scalar read/write, native monitored-item subscriptions | Server endpoint, security policy, and trusted certificates from the app store or the Configuration input |
| OPC UA PubSub | Publish and receive typed UADP key frames over UDP, optionally signed and encrypted | Local executor, IPv4 unicast or multicast, matching publisher/writer IDs, and configured field metadata; protected traffic also needs shared group keys and persistent executor state |
| DDS | Publish and receive typed topics with runtime schemas and bounded history | Local executor, reachable discovery network, matching domain/topic/type, and compatible QoS; certificate-based encryption requires Cyclone DDS 11 security libraries and OpenSSL 3 |
| MQTT | Client publish/subscribe and embedded broker | TCP or TLS; binary payloads and Last Will supported; embedded broker accepts clean sessions and QoS 0/1 |
| Sparkplug B | Protobuf codec, birth/data/death state, shared sequence tracking, host STATE, rebirth detection | MQTT transport and persisted birth/death sequence |
| NATS / JetStream | Core publish/subscribe; persistent JetStream publishing and durable pull consumers | Existing NATS server; JetStream streams must already exist |
| Kafka / Redpanda | Publish, consume, commit after successful handling | Local executor with access to all advertised brokers |
| RabbitMQ / AMQP 0-9-1 | Publish with broker confirmations, consume with manual acknowledgements | Existing broker, exchange, and queue routing |
| Redis Streams | XADD, consumer-group reads, acknowledgements | Standalone Redis; use a stable consumer name to replay its pending entries |
| Zenoh | Publish, subscribe, get, and query replies | Local executor with access to configured or discovered peers |
| Iroh | Authenticated peer connections, listen, send, receive | Local executor; both peers must use Flow-Like's `flow-like/messages/2` ALPN and framing |
| ADS / TwinCAT | Read/write and native notifications | Configured ADS route and AMS Net IDs |
| EtherNet/IP / CIP | Explicit Logix tag read/write and polling | Compatible Rockwell Logix controller; implicit Class 1 cyclic I/O is not implemented |
| EtherCAT | Native master, topology inspection, cyclic process data, output updates, startup CoE SDO writes | Linux or macOS, a dedicated Ethernet interface, and raw-interface permissions |
| PROFIBUS | Native DP-V0 master, parameterization/configuration, cyclic process data, Clear/Operate/Stop | Local serial PHY adapter, station timings, and configuration derived from device GSD files |
| PROFINET and cifX fieldbuses | Native controller process-image reads/writes, bus state, polling | Installed Hilscher cifX controller, driver, protocol firmware, and commissioned bus configuration |
| IO-Link | Master discovery, process data, indexed ISDU access, polling | Master implementing the [IO-Link JSON Integration 2.0 REST API](https://github.com/iolinkcommunity/JSON_for_IO-Link/tree/2.0.0) |
| HART | Commands and read-only command polling over serial, transparent TCP, or HART-IP v1 Token Passing | Local HART serial modem or compatible network endpoint |

The cifX nodes call the installed controller's native driver. The card firmware runs the fieldbus. They do not provide a portable software PROFINET controller or configure an arbitrary vendor's stack. Linux uses the supported LinuxCIFXDrv 3.x ABI; Windows uses the installed system driver. Supply absolute driver and firmware paths. The driver and protocol firmware are not bundled.

The embedded MQTT broker keeps retained messages in memory for its current run. It does not provide durable sessions or a broker account/ACL system. Use an existing MQTT broker when the deployment needs those features. The MQTT client also supports QoS 2 when connected to a broker that implements it. MQTT acknowledgements confirm transport receipt, independently of workflow handler completion.

Iroh Send waits for the receiving peer to read and acknowledge the complete frame. The acknowledgement precedes workflow handler completion. Protocol version 2 uses bidirectional streams for this exchange and cannot connect to version 1 peers; update both ends together.

IO-Link electrical master control requires a hardware-specific stack and is not implemented by the REST adapter. These distinctions matter when selecting hardware: a protocol name alone does not identify the controller role or supported transport.

ADS reads and writes are limited to 1 MiB. Incoming packets, including aggregated notifications, are limited to 16 MiB; an oversized packet closes the connection.

## Connect once, then use the session

The industrial Connect nodes return a session reference. Pass that reference to read, write, publish, consume, or disconnect requests within the same workflow run. Saving its JSON does not preserve a connection across runs or executor restarts. Cancellation removes the cached session and signals active consumers; Disconnect performs the protocol's close operation.

For example, **Connect Modbus** accepts:

```json
{
  "transport": "tcp",
  "endpoint": "192.0.2.10:502",
  "timeout_ms": 3000
}
```

Use its result as `session` in **Read Modbus**:

```json
{
  "session": { "ref_id": "<connect result>", "protocol": "modbus" },
  "request": {
    "unit_id": 1,
    "kind": "holding_registers",
    "start_address": 0,
    "count": 2,
    "timeout_ms": 3000
  }
}
```

Register addresses are zero-based. Use **Decode Sensor Registers** when the device encodes a number across multiple registers; byte order and word order must match its register map.

## Supply OPC UA client certificates

**Connect OPC UA** accepts a `certificates` object through its Configuration input. Supply the client application certificate, its matching private key, and the server certificates or certificate authorities you trust. The `application_uri` must match a URI in the client certificate's subject alternative names. Select `sign_and_encrypt` for encryption:

```json
{
  "endpoint": "opc.tcp://192.0.2.20:4840",
  "security_policy": "Basic256Sha256",
  "security_mode": "sign_and_encrypt",
  "timeout_ms": 10000,
  "certificates": {
    "application_uri": "urn:example:flow-like-client",
    "certificate_pem": "<client certificate PEM>",
    "private_key_pem": "<matching unencrypted RSA private key PEM>",
    "trusted_certificates_pem": ["<trusted server certificate or CA PEM>"],
    "issuer_certificates_pem": ["<intermediate CA PEM, if needed>"]
  }
}
```

Feed private keys from secret variables. The connection Configuration inputs for OPC UA, OPC UA PubSub, and DDS are marked sensitive. OPC UA validates the certificate/key pair and application URI before connecting, then checks the server's trust chain, hostname, and certificate validity. Supply intermediate CA certificates through `issuer_certificates_pem`; the current SDK expects the server's leaf certificate in its CreateSession response. Custom material uses a restricted temporary certificate store for that session and does not replace the app's store. Omitting `certificates` keeps the existing app-store behavior. The legacy **Read OPC UA Sensor** node accepts the same certificate object.

## Configure OPC UA PubSub peers

OPC UA PubSub sends DataSet messages independently of an OPC UA client session. Configure the same publisher ID type and value, writer-group ID, DataSet writer ID, field order, and field types on both peers. Field names are local labels; UADP key frames carry values in the configured order. When a configuration version is supplied, the subscriber requires both version numbers to match.

The adapter supports IPv4 unicast and multicast with the binary UADP message format. Publications contain one Variant-encoded key frame. Supported values include Boolean, integer, finite floating-point, String, DateTime, ByteString, and StatusCode fields. Arrays, structured values, RawData, delta frames, metadata discovery, and the MQTT/JSON transport are outside this implementation. Unsupported encodings in matching messages produce an error. Use OPC UA client nodes when the device requires client/server subscriptions.

For example, connect a publisher to a peer listening on `192.0.2.20:4840`:

```json
{
  "address": "opc.udp://192.0.2.20:4840",
  "interface": "0.0.0.0",
  "mode": "publisher",
  "writer": {
    "publisher_id": { "type": "uint32", "value": 100 },
    "writer_group_id": 10,
    "data_set_writer_id": 1
  },
  "fields": [{ "name": "temperature", "data_type": "double" }],
  "timeout_ms": 3000,
  "max_datagram_bytes": 1400
}
```

Publish with the returned session and `values: [{ "type": "double", "value": 23.5 }]`. On the receiving executor, use `mode: "subscriber"` with the same address, writer, and fields. For multicast, supply the group address and select the local network interface; multicast traffic has a TTL of one. Receive returns one matching message; Subscribe invokes the referenced handler for each message and exits with a timeout error if no new matching message arrives within `timeout_ms`. UDP provides no delivery acknowledgement or replay, and packets larger than the network MTU may fragment.

To protect messages, add a `security` object to the Configuration input. Its `mode` is `sign` or `sign_and_encrypt`; `security_token_id` is a nonzero integer. Supply `signing_key` and `encrypting_key` as distinct 32-byte arrays, and `key_nonce` as a four-byte array. Both peers must use matching values. This uses the standard PubSub-Aes256-CTR policy with HMAC-SHA256 signatures. PubSub uses symmetric group keys; X.509 certificates alone do not encrypt its UDP messages. Keys are provisioned through the input; automatic Security Key Service retrieval is not implemented.

Protected sessions reject plaintext, tampered messages, and replayed messages. The executor persists nonce and replay counters in app storage. Publishers share counters across an app; each subscriber Connect node keeps its own replay history. A second active connection from the same subscriber node is rejected. Copying that node starts a separate replay history. The Rust adapter requires an explicit persistent directory, with a separate directory per logical subscriber; the catalog supplies these paths automatically.

Keep that state across restarts. Provision different encryption keys and key nonces for independent app or executor stores. Rotate both whenever state is deleted, restored, or cloned; local storage cannot detect a rollback from an old backup. Changing only the token ID does not reset the nonce counter.

## Configure DDS topics

Data Distribution Service (DDS) discovers readers and writers in a domain and matches their topic, type, and quality-of-service (QoS) settings. Each connection creates one publisher or subscriber endpoint. Supply the peer's exact type name, field names, declaration order, primitive types, and key annotations. The adapter builds the DDS type from this schema at runtime.

For a peer using `@final struct Telemetry { double value; };`, connect with:

```json
{
  "domain_id": 0,
  "topic": "Telemetry",
  "data_type": {
    "name": "Telemetry",
    "fields": [{ "name": "value", "data_type": "float64" }]
  },
  "role": "publisher",
  "reliability": "reliable",
  "history_depth": 32,
  "timeout_ms": 10000
}
```

The matching subscriber uses `role: "subscriber"`. Publish a JSON object whose fields match the configured schema, such as `{"value": 23.5}`. Publication waits for a matching reader. Reliable publication also waits for transport acknowledgement; this does not confirm that its workflow handler succeeded. Best-effort delivery can lose samples. Discovery requires reachable UDP peers on a local executor.

Supported types are final structs with sequential member IDs, primitive fields, strings, and unbounded IDL sequences of those types. Set `sequence: true` for a sequence field and `key: true` for a scalar integer, Boolean, or string key. Subscribers require matching DDS XTypes identifiers; type coercion and peers without type information are rejected. Nested structs, unions, enums, fixed arrays, optional members, and arbitrary IDL import are not supported. The wire representation is XCDR1. Reliability is configurable; durability is volatile, so a late subscriber receives future publications rather than retained history.

History holds up to `history_depth` samples in total, with a maximum of 256. Published and returned samples are limited to 1 MiB of JSON and each sequence to 65,536 elements. Incoming limits are checked after SDK decoding; they do not bound the SDK's network-buffer allocations. Plain DDS type descriptors are cached for the executor process lifetime; the adapter reuses identical schemas and rejects a 129th distinct schema. Restarting the executor clears that cache. Receive reports instance lifecycle notifications with `valid_data: false` and no data object.

For certificate-based encryption, add `security` with `identity_ca_pem`, `identity_certificate_pem`, `private_key_pem`, `permissions_ca_pem`, `governance_signed`, and `permissions_signed`. Supply an optional `private_key_password` for an encrypted private key. The two signed documents are S/MIME DDS Security governance and permissions documents, signed by your permissions CA. Permissions must authorize the certificate's subject for the requested domain, topic, and publisher/subscriber role. Governance must require authentication and encryption; permissive policies are rejected.

This secure mode supports domain IDs 0 through 230 and requires a 64-bit executor, Cyclone DDS 11 and its authentication, access-control, and cryptographic plugins, with OpenSSL 3. Install those native runtime libraries on the executor before connecting. The application loads them at runtime, so metadata builds do not require them. Missing libraries, invalid certificates, or rejected policies produce an error; the connection never falls back to plain DDS. Omitting `security` uses the existing Rust DDS backend.

Secure sessions sharing a domain in one executor must supply identical identity CA, permissions CA, and signed governance documents. The adapter owns that Cyclone domain and disables diagnostic tracing, because native discovery traces can include credentials. It ignores `CYCLONEDDS_URI`, including its network settings, and rejects a domain already owned by another Cyclone user in the same process. Regular native error messages remain available.

## Run a listener from a daemon event

Start the workflow with a daemon event, connect, and enter a Subscribe, Consume, or Poll node. Reference exactly one handler function. Give that function a non-execution output named `event` to receive the full event object, or outputs named after individual event fields. The node's input and output schemas describe those fields.

The listener waits for each handler invocation to finish before processing the next delivery. JetStream, RabbitMQ, Redis Streams, and Kafka acknowledge or commit only after successful handling. Handler failure leaves the message unacknowledged. A crash between the handler's external side effect and the acknowledgement can still cause duplicate delivery, so use the broker message ID or your own business key when an operation must be idempotent.

Use consumer timeouts and bounded queues to match processing capacity. Core pub/sub messages are transient. Redis replays pending entries belonging to the configured consumer; it does not automatically claim abandoned entries from other consumers. Broker retention, queue bindings, stream creation, and recovery policy remain deployment configuration.

EtherCAT and PROFIBUS run their bus cycles on dedicated workers. Workflow handlers receive snapshots; slow workflow execution does not set the bus-cycle frequency. EtherCAT validates working counters and slave state. The executor does not promise hard real-time scheduling, and successful software tests do not establish timing or device interoperability for a physical installation.

EtherCAT Stop and failed startup request INIT and report cleanup failures. PROFIBUS Disconnect enters Clear and waits up to one second for a fresh data-exchange acknowledgement from every configured station. If stations do not acknowledge, Disconnect reports their addresses instead of claiming that Clear delivery completed.

## Publish Sparkplug B over MQTT

Sparkplug nodes build messages and state explicitly. The workflow owns publication order and durable state:

1. Load the previous `bd_seq`, then call **Prepare Sparkplug Session** with the group, edge-node ID, metric definitions, and timestamp. Persist the returned `state.bd_seq` before opening a new MQTT connection.
2. Put the returned `last_will` into MQTT connection configuration and set `clean_session` to `true`. Map numeric QoS `0`, `1`, or `2` to `AtMostOnce`, `AtLeastOnce`, or `ExactlyOnce` for MQTT nodes.
3. Connect MQTT, apply the `birth` action with **Sparkplug State Transition**, and publish each returned message in order. Feed its byte payload to MQTT's binary payload input. Use the returned state for the next transition.
4. Publish device births before their data. Node and device data share one sequence counter. Metric names, aliases, and datatypes must agree with the birth definitions.
5. Subscribe to commands and apply `birth` when a valid rebirth command is received. **Observe Sparkplug State** tracks an individual edge node and reports sequence gaps that require rebirth; it does not send the rebirth command itself.
6. For an orderly shutdown, publish the `death` transition before disconnecting MQTT. The broker sends the installed Last Will on an unexpected connection loss.

The protobuf codec uses protobuf JSON field names. Represent 64-bit integer values as decimal strings, for example `{"name":"count","datatype":8,"longValue":"123"}`. Birth and data transitions require each value field to match the birth datatype, including updates that omit `datatype`. For example, a Double metric uses `doubleValue`; array metrics use packed bytes in `bytesValue`. Host STATE messages use JSON on `spBv1.0/STATE/<host-id>` with `online` and `timestamp`, QoS 1, and retention.

## Executor access and builds

Serial ports, raw Ethernet, peer discovery, and native driver loading run on local or desktop executors. Kafka requires local execution because brokers can redirect clients to advertised addresses. In server execution, NATS, Redis, AMQP, and MQTT over TLS require IP-literal endpoints that pass the server's egress policy; TLS certificates must cover that address. MQTT over TLS currently accepts IPv4 literals and rejects IPv6 literals because of its SDK's hostname handling. Plain MQTT pins the approved resolved address. Local execution also supports TLS hostnames. HTTP-based IO-Link uses the guarded HTTP client.

The Rust adapters live in `flow-like-industrial`; node registration and execution integration live in `flow-like-catalog-industrial`. Neither requires the ML training engines. Product execution bundles enable them by default; metadata-only builds avoid linking the optional protocol SDKs. Existing inspection sensor node IDs are preserved, and `flow-like-ml-sensors` forwards its Rust API to the industrial crate. GenICam remains a separate `sensor-genicam` opt-in.

## Run protocol interoperability tests

With Docker running and the repository's Rust build dependencies installed, run:

```sh
bash tools/test-industrial-e2e.sh --all
```

The runner builds dedicated fixtures, checks their readiness, runs the ignored integration tests, and removes its containers and volumes on exit. It binds test services to local ports; stop an earlier test run before starting another. Use `--adapters` for network adapter tests, `--mqtt` for MQTT node tests, or `--devices` for the Linux camera, Iroh peer, EtherCAT slave, Modbus RTU, OPC UA PubSub, and DDS harnesses.

Use `--pubsub` to run only the OPC UA PubSub and DDS tests, including their security cases. Use `--security` to run those suites together with OPC UA client certificate tests. PubSub and DDS adapters and independent peers share a container network so UDP traffic does not depend on Docker Desktop port forwarding. The EtherCAT container uses `NET_ADMIN` and `NET_RAW` capabilities to create its own virtual Ethernet pair; it does not use a host network interface.

| Coverage | Reference peer or contract |
|---|---|
| Modbus TCP / RTU, OPC UA | PyModbus TCP and serial servers, plus asyncua; RTU uses a private virtual serial pair |
| OPC UA PubSub | open62541 UADP publisher and subscriber |
| DDS | Cyclone DDS publisher and subscriber with an independently compiled IDL type |
| MQTT, NATS, Redis Streams, AMQP, Kafka | Mosquitto, NATS, Redis, RabbitMQ, and Redpanda |
| Explicit Logix CIP, ADS | libplctag's `ab_server` and Beckhoff's ADS router/server samples |
| Sparkplug B | Eclipse Tahu Python codec and helpers over Mosquitto, in both directions |
| Zenoh | Official router, memory storage, and a separate Python peer |
| HART-IP v1 | FieldComm Group's server with deterministic values supplied through its application interface |
| GigE Vision / GenICam | Aravis fake camera, including actual image packets |
| Iroh | Separate raw SDK peer implementing Flow-Like's framing and acknowledgements |
| EtherCAT | KickCAT slave emulator over a private virtual Ethernet pair |
| IO-Link JSON v2 | Official OpenAPI schema and examples, enforced by Prism |

These tests exercise adapters over sockets, including errors and connection cleanup. They do not run every workflow graph or certify protocol conformance. Zenoh and Iroh peers share their upstream transport SDKs. IO-Link validation is stateless API contract coverage, so it cannot establish master behavior or physical device readback. The Sparkplug tests cover the implemented codec and state transitions, rather than the full Sparkplug certification suite.

Physical serial links, EtherCAT, PROFIBUS, and cifX-backed buses still need suitable adapters, controllers, and devices. Docker results do not establish electrical behavior, bus timing, controller firmware compatibility, or vendor-specific device support. Serial HART and independent PROFIBUS slave behavior remain outside this Docker suite. Validate those properties on the target hardware before relying on an installation.
