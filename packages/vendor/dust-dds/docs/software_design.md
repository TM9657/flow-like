# Dust DDS: Software design document

## Introduction
Data Distribution Services (DDS) is a middleware protocol and API standard for data-centric connectivity. The main goal of DDS is to share the right data at the right place at the right time, even between time-decoupled publishers and subscribers. Dust DDS is the [S2E Software Systems](https://www.s2e-systems.com) implementation of the [Data Distribution Services (DDS)](https://www.omg.org/omg-dds-portal/) and [Real-time Publisher-Subscriber (RTPS)](https://www.omg.org/spec/DDSI-RTPS/About-DDSI-RTPS/) protocols using the [Rust programming language](https://www.rust-lang.org/).

This Software Design Document (SDD) outlines the architectural design, key design choices and trade-offs related to the development of Dust DDS. This document is intended for those who are interested in the internal workings of Dust DDS and to keep track of important design decisions during the development process. It is not meant as a documentation of actual functionality which is instead done in the source code itself using cargo docs. For more high-level information and usage examples please refer to the [README](../README.md).

## High-level architecture

The implementation of Dust DDS is done in what could be better described as layers which implement different parts of the functionality. Looking at it from a Dust DDS library user the layers can be described as follows:

1. __DDS API Layer__
The DDS API layer serves as the fundamental interface for Dust DDS, implementing the specifications defined by the Object Management Group (OMG) for the Data Distribution Service (DDS). This layer exposes the standardized DDS functionalities to users, allowing them to interact with Dust DDS using well-defined DDS concepts and operations. It acts as the bridge between the user's application and the underlying data distribution infrastructure. Much of the functionality implemented in this layer revolves around communicating with the different actors.

2. __Actors__
The Dust DDS implementation follows the actor model. In short, each class is typically wrapped into an actor object and the methods are invoked by sending a message using a channel.  Actors may modify their own private state, but can only affect each other indirectly through messaging (removing the need for lock-based synchronization). More information on actor can be found on [Wikipedia](https://en.wikipedia.org/wiki/Actor_model) or [on this blog article focusing on Rust](https://ryhl.io/blog/actors-with-tokio/).

3. __RTPS__
The RTPS (Real-time Publish-Subscribe) layer implements the Object Management Group's RTPS standard. This layer includes types, traits, and functionality that allow using the RTPS protocol. This is needed for interoperable data exchange in real-time and distributed systems.

## Implementation details

In this section we describe the design decision and some trade-offs and technical limitations analysis. For Dust DDS we are using only Stable Rust with unsafe code explicitly forbidden. At the time of writting we are using Rust edition 2021 with compiler version 1.73.0.

### DDS API layer

The DDS API Layer in Dust DDS adheres closely to the Object Management Group (OMG) DDS standard, which specifies DDS API methods as interfaces. In Rust, the natural mapping for such IDL interfaces would be traits. However, due to certain technical limitations, Dust DDS implements the DDS API methods as inherent functions of objects. This sections details some of the possibilities considered and the limitation and trade-off analysis.

1. __Returning Opaque Types in Traits__
Opaque types are types whose details are hidden from the user and are only accessible through a defined set of functions or methods. They do not require dynamic dispatch and it would therefore be a good choice for this type of API. Lets consider the following simplified FooDataWriter trait

```rust
trait FooDataWriter<Foo> {
    fn write(&self, data: &Foo) -> Result<(),()>;
}

```

The DDS standard defines that a Publisher can create Data Writers for any different *Foo* type for which data can be distributed using the middleware. This creation in a simplified form looks something like this:

```rust
trait Publisher {
    fn create_datawriter<Foo>(&self) -> impl FooDataWriter<Foo>;
}
```

This however fails to compile with `error[E0562]: impl Trait only allowed in function and inherent method return types, not in trait method return types`. An alternative to this could be to use an associated type:

```rust
trait Publisher {
    type DataWriter<Foo>;
    fn create_datawriter<Foo>(&self) -> Self::DataWriter<Foo>;
}

impl Publisher for PublisherNode {
    type DataWriter<Foo> = impl FooDataWriter<Foo>;
    fn create_datawriter<Foo>(&self) -> Self::DataWriter<Foo> {
        todo!()
    }
}
```

However, having opaque types return from associated types is still unstable `error[E0658]: impl Trait in associated types is unstable`.

2. __Trait objects with generic methods__
The API could instead return trait objects and do dynamic dispatch which for the usages of Dust DDS would likely have an irrelevant impact. The publisher trait could then be modified to:

```rust
trait Publisher {
    fn create_datawriter<Foo>(&self) -> Box<dyn FooDataWriter<Foo>>;
}
```

The following level is the *Domain Participant* which creates a *Publisher* in the following simplified manner:

```rust
trait DomainParticipant {
    fn create_publisher(&self) -> Box<dyn Publisher>;
}
```

This results in the error `error[E0038]: the trait Publisher cannot be made into an object. ... note: for a trait to be "object safe" it needs to allow building a vtable to allow the call to be resolvable dynamically;`

Taking into account these constraints, at this point the decision is either to have a struct implementing the DDS API methods directly on have the trait return a mix of concrete types and trait objects.

### Type support traits

The DDS standard allows the user to communicate any data type of its choice so long as it provides the information necessary for the system to handle the data dissemination for that type. This includes information on how to serialize and deserialize the data as well as the necessary key definition that will allow the Service to distinguish different instances of the same type. We refer as Foo to any data type that can be transmitted over DDS. We have identified the following functionalities that needs to be provided by Foo to allow it to be transmitted:

1. *Serialize data* - This is the operation that allows converting a specific instance of Foo into bytes to be possibly transmitted over the wire. This is represented by the `DdsSerialize` trait
2. *Deserialize data* - This is the operation that allows converting the received bytes into Foo possibly borrowing bytes from the history cache. This is represented by the `DdsDeserialize<'de>` trait
3. *Serialize key* - This is the operation that allows serialize the key fields of Foo into bytes to be possily transmitted over the wire (e.g. when unregistering or disposing an instance). This is represented by the `DdsSerializeKey` trait.
4. *Get key from Foo* - This is the operation that allows computing a key represent a specific instance of the type Foo. An instance is identified by its key fields so this operation is similar to serializing the key but the result is not intended to be transmitted over the wire so this operation is kept separate and independent. This is represented by the `DdsGetKeyFromFoo` trait.
5. *Get key from serialized data* - This is the operation that allows retrieving the key from the serialized Foo. This is used to identify the instance a specific Data submessage with data refers to when received on the Data Reader side. This is represented by the `DdsGetKeyFromSerializedData` trait.
6. *Get key from serialized key fields* - This is the operation that allows retrieving the key from the serialized Foo key fields. This is used to identify the instance a specific Data submessage with only key (e.g. for unregister or dispose) refers to when received on the Data Reader side. This is represented by the `DdsGetKeyFromSerializedKeyFields` trait.
7. *Has key* - This is the operation that allows determining whether a type is Keyed or not. This is represented by the `DdsHasKey` trait.

Any type Foo implementing these traits can be used to create a DataWriter<Foo> and DataReader<Foo> and have its information communicated using Dust DDS.

### Listeners

The DDS standard provides a listener mechanism which can be used to implement an event-based response system. Each DDS entity has a listener associated with it with the functionality described by a listener interface. According to the standard it is permitted to use 'nil' as the value of the listener. The 'nil' listener behaves as a Listener whose operations perform no action.

This snippet gives the exploration of the design space for the listener interface implementation:

```rust
    pub trait AnyListener{}

    pub trait GenericListener<Foo>{}

    impl<Foo> AnyListener for Box<dyn GenericListener<Foo>> {}

    pub trait AssociatedTypeListener{ type Foo;}

    impl<Foo> AnyListener for Box<dyn AssociatedTypeListener<Foo=Foo>> {}

    impl<T> AnyListener for T where T: AssociatedTypeListener{}

    pub struct DataWriter {
        listener: Option<Box<dyn AnyListener>>,
    }

    fn create_datawriter_with_box_generic<Foo>(listener: Option<Box<dyn GenericListener<Foo>>> ) -> DataWriter
        where Foo:'static
    {
        DataWriter {listener: listener.map::<Box<dyn AnyListener>,_>(|l| Box::new(l))}
    }

    fn create_datawriter_with_box_associated_type<Foo>(listener: Option<Box<dyn AssociatedTypeListener<Foo=Foo>>> ) -> DataWriter
        where Foo:'static
    {
        DataWriter {listener: listener.map::<Box<dyn AnyListener>,_>(|l| Box::new(l))}
    }

    // Not possible to box this because AnyListener can not be implemented "for <Foo>"
    // fn create_datawriter_with_impl_generic<Foo>(listener: Option<impl GenericListener<Foo>> ) -> DataWriter
    // {
    //     DataWriter {listener: listener.map::<Box<dyn AnyListener>,_>(|l| Box::new(l))}
    // }

    // Only implementations which doesn't require a Foo: 'static
    fn create_datawriter_with_option_impl_associated_type<Foo>(listener: Option<impl AssociatedTypeListener<Foo=Foo> + 'static> ) -> DataWriter
    {
        DataWriter {listener: listener.map::<Box<dyn AnyListener>,_>(|l| Box::new(l))}
    }

    fn create_datawriter_with_impl_associated_type<Foo>(listener: impl AssociatedTypeListener<Foo=Foo> + 'static ) -> DataWriter
    {
        DataWriter {listener: Some(Box::new(listener))}
    }

    fn main() {
        // Uncomment for compilation error about type inference
        // create_datawriter_with_option_impl_associated_type(None);


        println!("Test compilation of different listener combos")
    }
```

Using an `Option<Box<Listener>>` as the input argument on the listener has an easy way to give an empty listener but requires Foo to have a `'static` lifetime bound. Having a generic `impl Listener` fits better with the standard definition of for example the creation of data writer `DataWriter create_datawriter(in Topic a_topic, in DataWriterQos qos, in DataWriterListener a_listener, in StatusMask mask);` but the listener type must be always defined.

To allow the user to easily install the "nil" listener a convinience struct called NO_LISTENER is created which implements all the Listener traits. The struct is defined as `pub struct NO_LISTENER<Foo = ()>(PhantomData<Foo>);` with a default empty type to allow usage in cases where Foo is not needed (e.g. Participant listener).