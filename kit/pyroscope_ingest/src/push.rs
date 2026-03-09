// Push API protobuf types, copied from src/encode/gen/push.rs and src/encode/gen/types.rs.
// These match the Pyroscope push.v1.PusherService/Push protobuf schema.

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushRequest {
    #[prost(message, repeated, tag = "1")]
    pub series: ::prost::alloc::vec::Vec<RawProfileSeries>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RawProfileSeries {
    #[prost(message, repeated, tag = "1")]
    pub labels: ::prost::alloc::vec::Vec<LabelPair>,
    #[prost(message, repeated, tag = "2")]
    pub samples: ::prost::alloc::vec::Vec<RawSample>,
}

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct RawSample {
    #[prost(bytes = "vec", tag = "1")]
    pub raw_profile: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "2")]
    pub id: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct LabelPair {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub value: ::prost::alloc::string::String,
}
