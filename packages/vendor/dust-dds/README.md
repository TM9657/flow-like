# Dust DDS

[![CircleCI](https://dl.circleci.com/status-badge/img/gh/s2e-systems/dust-dds/tree/main.svg?style=svg)](https://dl.circleci.com/status-badge/redirect/gh/s2e-systems/dust-dds/tree/main)
[![Crates.io](https://img.shields.io/crates/v/dust_dds.svg)](https://crates.io/crates/dust_dds)
[![Docs.rs](https://docs.rs/dust_dds/badge.svg)](https://docs.rs/dust_dds)
[![License](https://img.shields.io/crates/l/dust_dds.svg)](https://github.com/s2e-systems/dust-dds/blob/master/LICENSE)

[Dust DDS](https://dust-dds.s2e-systems.com) is a native Rust implementation of the [Data Distribution Service (DDS)](https://www.omg.org/omg-dds-portal/) middleware using the [Real-time Publisher-Subscriber (RTPS)](https://www.omg.org/spec/DDSI-RTPS/About-DDSI-RTPS/) wire protocol developed by [S2E Software Systems](https://www.s2e-systems.com).

This crate provides a Rust implementation of the minimum DDS profile. It uses only stable Rust and has no `unsafe` code while providing a large code coverage validated by our CI systems to ensure its quality.


## Example

A basic example on how to use Dust DDS. The publisher side can be implemented as:

```rust
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{listener::NO_LISTENER, qos::QosKind, status::NO_STATUS, type_support::DdsType},
};

#[derive(DdsType)]
struct HelloWorldType {
    #[dust_dds(key)]
    id: u8,
    msg: String,
}

let domain_id = 0;
let participant_factory = DomainParticipantFactory::get_instance();

let participant = participant_factory
    .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
    .unwrap();

let topic = participant
    .create_topic::<HelloWorldType>("HelloWorld", "HelloWorldType", QosKind::Default, NO_LISTENER, NO_STATUS)
    .unwrap();

let publisher = participant
    .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
    .unwrap();

let writer = publisher
    .create_datawriter::<HelloWorldType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
    .unwrap();

let hello_world = HelloWorldType {
    id: 8,
    msg: "Hello world!".to_string(),
};
writer.write(hello_world, None).unwrap();
```

The subscriber side can be implemented as:

```rust
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{listener::NO_LISTENER, qos::QosKind, status::NO_STATUS, type_support::DdsType},
};

#[derive(Debug, DdsType)]
struct HelloWorldType {
    #[dust_dds(key)]
    id: u8,
    msg: String,
}

let domain_id = 0;
let participant_factory = DomainParticipantFactory::get_instance();

let participant = participant_factory
    .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
    .unwrap();

let topic = participant
    .create_topic::<HelloWorldType>("HelloWorld", "HelloWorldType", QosKind::Default, NO_LISTENER, NO_STATUS)
    .unwrap();

let subscriber = participant
    .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
    .unwrap();

let reader = subscriber
    .create_datareader::<HelloWorldType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
    .unwrap();

if let Ok(hello_world_sample) = reader.read_next_sample() {
    println!("Received: {:?}", hello_world_sample.data.unwrap());
}
```

## A brief introduction to DDS

DDS is a machine-to-machine communication middleware and API standard designed for data-centric connectivity. At its core, DDS aims to facilitate the seamless sharing of pertinent data precisely where and when it's needed, even across publishers and subscribers operating asynchronously in time. With DDS, applications can exchange information through the reading and writing of data-objects identified by user-defined names (Topics) and keys. One of its defining features is the robust control it offers over Quality-of-Service (QoS) parameters, encompassing reliability, bandwidth, delivery deadlines, and resource allocations.

The [DDS standard](https://www.omg.org/spec/DDS/1.4/PDF) "defines both the Application Interfaces (APIs) and the Communication Semantics (behavior and quality of service) that enable the efficient delivery of information from information producers to matching consumer". Complementing this standard is the [DDSI-RTPS specification](https://www.omg.org/spec/DDSI-RTPS/2.5/PDF), which defines an interoperability wire protocol for DDS. Its primary aim is to ensure that applications based on different vendors' implementations of DDS can interoperate. The implementation of Dust DDS primarily centers around the DDS and DDSI-RTPS standards.

## Who should use DDS?

Choosing the right middleware depends on many factors like *network architecture*, *latency requirements*, *scalability* and *workload*. There are though some questions you can ask to determine if DDS is the right solution:
1. Do I have a dynamic network topology? A dynamic network topology means that devices are able to join and leave the communication domain at anytime. Imagine an air traffic control system, a rail network monitoring system or a robot with variable sensor and actuator setup. This makes any fixed configuration very challenging and is well handled by the dynamic discovery system of DDS.
2. Do I need to send a large volume of messages and/or operate on the millisecond time range? Imagine publishing data from a radar system, monitoring a vehicle position or sending mixed robot video and sensor data. Default DDS implementation use UDP and provide an extensive QoS which enable it to handle high-throughput and low-latency messaging.
3. Does my system need to be robust to failures on the different nodes? Imagine a defense system which should be resilient or a robot that can continue to operate when certain sensors or actuators are removed. DDS discovery does not rely on any centralized server and communication is peer-to-peer. This reduces single points-of-failure and bottlenecks.
4. Do I need well defined communication types that can evolve over time? Imagine an automotive or robotic system where you want to have well defined data types transmitted by the sensors. DDS is a data-centric middleware and its type system allows type definitions to be extended and evolve over time.

If you answered yes to one or more of these questions, DDS is likely a strong candidate for your application. There are also certain common applications for which DDS is typically not the best fit. These include:

1. Heavy database-centric workloads. If your system is primarily used to store and retrieve data from persistent data storage you are probably better off with a database-driven approach.
2. Non-real-time web applications. If your main requirement is scalable centralized web-based communication you are probably better of with other message broker solutions
3. Simple request-response workloads. If your application follows a standard request-response model then you are probably better off with REST or RPC-based communication even if DDS is able to handle request reply mechanisms.

## Rust type definition using #[derive(DdsType)]

This library provides the `DdsType` derive macro to support defining DDS types in Rust. The `DdsType` derive macro can be applied to `struct`s, tuples, and `enum`s to automatically implement the traits needed for communication.

### Container Attributes

Container attributes apply to the entire `struct` or `enum` definition and are defined using `#[dust_dds(...)]` on the container:

* **`name = "<string>"`**:
  Customizes the name of the DDS type. Defaults to the Rust struct or enum identifier.
* **`extensibility = "final" | "appendable" | "mutable"`**:
  Specifies the type extensibility (defined in the DDS XTypes specification). Defaults to `"final"`.
* **`nested`**:
  A boolean flag marking the type as nested. Nested types cannot be used as standalone top-level topics but can be members of other types.

### Struct Field Attributes

Field attributes apply to individual fields of a struct or tuple, defined using `#[dust_dds(...)]` on the field:

* **`key`**:
  Marks the field as part of the key of the type. Multiple fields can be marked as keys.
* **`id = <integer>`**:
  Explicitly assigns a member ID. This is particularly useful for `"mutable"` extensibility.
* **`optional`**:
  Marks the field as optional.
* **`default_value = <expression>`**:
  Specifies a default value expression to use if the field is missing during deserialization.
* **`non_serialized`**:
  Marks a field to be skipped during serialization and deserialization. It is initialized using `Default::default()`.
* **`external`**:
  Marks the field as external, which indicates it uses a pointer/box/reference semantic representation in the DDS type system.
* **`hashid`**:
  Instructs the macro to automatically assign a member ID computed from the MD5 hash of the field's name (first 4 bytes as a little-endian integer).

### Enum Attributes

An enum can represent either a standard **Enumerated Type** or an **XTypes Union**.

#### 1. Enumerated Types (Unit Variants Only)

If an enum contains only unit variants (variants without fields), it is mapped to a DDS enumerated type.

* **`bit_bound = "8" | "16" | "32"`**:
  Specifies the size of the integer representation in bits. Defaults to `"32"`.
* Custom integer discriminants (e.g. `A = 10`) are mapped directly to their corresponding DDS values.

#### 2. XTypes Unions (Variants with Fields)

If an enum contains at least one variant with fields, it is mapped to a DDS Union.

* **`switch(<Type>)` or `switch(key, <Type>)`**:
  **Required.** Specifies the type of the discriminator. If `key` is present, the discriminator is treated as a key.
* **`case = <expression>`** (on variants):
  Specifies the discriminator value(s) for which this variant is selected. If omitted, defaults to the 0-indexed index of the variant.
* **`default`** (on a variant):
  Marks the variant as the default case if no other case matches.

### Examples

#### Basic Struct with Key

```rust
use dust_dds::infrastructure::type_support::DdsType;

#[derive(DdsType)]
struct HelloWorld {
    #[dust_dds(key)]
    id: i32,
    msg: String,
}
```

#### Appendable and Nested Struct

```rust
use dust_dds::infrastructure::type_support::DdsType;

#[derive(DdsType)]
#[dust_dds(name = "CustomPoint", extensibility = "appendable", nested)]
struct Point {
    x: f64,
    y: f64,
}
```

#### Mutable Struct with Custom Member IDs and Defaults

```rust
use dust_dds::infrastructure::type_support::DdsType;

#[derive(DdsType)]
#[dust_dds(extensibility = "mutable")]
struct Profile {
    #[dust_dds(id = 1, key)]
    id: u32,
    #[dust_dds(id = 2)]
    username: String,
    #[dust_dds(id = 10, optional)]
    email: Option<String>,
    #[dust_dds(id = 11, default_value = 18)]
    age: u32,
    #[dust_dds(non_serialized)]
    session_count: u32,
}
```

#### Enumerated Type with Bit Bound

```rust
use dust_dds::infrastructure::type_support::DdsType;

#[derive(DdsType)]
#[dust_dds(bit_bound = "16")]
enum TrafficLight {
    Red = 1,
    Yellow = 2,
    Green = 3,
}
```

#### Union with Discriminator Switch

```rust
use dust_dds::infrastructure::type_support::DdsType;

#[derive(DdsType)]
#[dust_dds(switch(i32))]
enum Shape {
    #[dust_dds(case = 1)]
    Circle(f64),
    #[dust_dds(case = 2)]
    Square { side: f64 },
    #[dust_dds(default)]
    Unknown,
}
```

## IDL type generation

If using different programming languages or vendors, the DDS type can be generated from an IDL file using the [dust_dds_gen crate](https://crates.io/crates/dust_dds_gen). For example, the `HelloWorldType` can be generated from the following IDL:

```idl
struct HelloWorldType {
  @key
  uint8 id;
  string msg;
};
```

## Sync and Async library API

Dust DDS provides both a "sync" and an "async" API to allow integrating DDS in the largest number of applications with maximum performance. In general, the first option should be to use the sync API and make use of the DDS specified functionality such as listeners for event based programs.

When implementing applications that already make use of async, then the async API must be used. In particular, when using a Tokio runtime, using the Sync API will result in a panic due to blocking calls. You can see find an example in the examples folder.

## Dust DDS extensions

### Dust DDS C Bindings

Dust DDS provides [C bindings](https://dust-dds.s2e-systems.com/product/c-binding/) as an extension, enabling integration with existing C and C++ codebases and facilitating adoption in environments where Rust cannot be used directly. The bindings expose a stable, C-compatible API that mirrors the core DDS concepts while preserving Dust DDS’s focus on correctness and performance. This extension makes it possible to incrementally introduce Dust DDS into legacy systems, mixed-language projects, or constrained embedded environments, without requiring a full rewrite in Rust.

### Dust DDS for microcontrollers

Dust DDS can be deployed on [microcontrollers](https://dust-dds.s2e-systems.com/product/microcontrollers/) and other *no_std* targets, enabling DDS-based communication in highly constrained environments. For this purpose, Dust DDS provides a pre-made runtime built on top of the Embassy async framework, offering a ready-to-use execution model tailored to embedded systems. In addition, S2E offers services to assist with porting Dust DDS to custom or proprietary runtimes, allowing integration into environments where Embassy is not available or where specific platform requirements apply.

### DDS over the Internet

Standard DDS implementations are limited to local networks because they rely on UDP multicast for participant discovery, which is not permitted across the internet. If you want to connect your devices over the internet, our [Global DDS](https://dust-dds.s2e-systems.com/product/global-dds/) offers a plug-and-play solution without code modifications or additional software needed.

### DDS REST API

If you want to interact with your DDS data using a REST API you can use our [Nebula DDS WebLink](https://www.s2e-systems.com/products/nebula-dds-weblink/) software. Nebula DDS WebLink provides a server implementing the Object Management Group (OMG) Web-Enabled DDS v1.0 standard.

## Shapes demo

DDS interoperability is typically tested using a shapes demo. The Dust DDS Shapes Demo is available on our repository and can be started by running `cargo run --package dust_dds_shapes_demo` from the root folder.

![Dust DDS Shapes demo screenshot](https://raw.githubusercontent.com/s2e-systems/dust-dds/master/dds/docs/shapes_demo_screenshot.png)

## Release schedule

Dust DDS doesn't follow a fixed release schedule but we will make releases as new features are implemented.

## License

This project is licensed under the Apache License Version 2.0.
