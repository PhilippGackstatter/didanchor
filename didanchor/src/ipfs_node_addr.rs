use multiaddr::{Multiaddr, Protocol};
use url::Url;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpfsNodePublicAddress {
    #[serde(with = "protocol_serialization")]
    pub host: Protocol<'static>,
    pub swarm_port: Multiaddr,
    #[serde(with = "protocol_serialization")]
    pub gateway_port: Protocol<'static>,
    #[serde(with = "protocol_serialization")]
    pub peer_id: Protocol<'static>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IpfsNodeManagementAddress {
    pub host: String,
    pub api_port: u16,
    pub cluster_port: u16,
}

impl IpfsNodeManagementAddress {
    pub fn to_cluster_address(&self) -> anyhow::Result<Url> {
        Url::parse(&format!(
            "http://{host}:{cluster_port}",
            host = self.host,
            cluster_port = self.cluster_port
        ))
        .map_err(Into::into)
    }

    pub fn to_api_address(&self) -> anyhow::Result<Url> {
        Url::parse(&format!(
            "http://{host}:{api_port}",
            host = self.host,
            api_port = self.api_port
        ))
        .map_err(Into::into)
    }
}

pub(crate) mod protocol_serialization {
    //! Provides serialization for the Protocol as a string.

    use multiaddr::Protocol;
    use serde::de::Visitor;
    use serde::de::{self};
    use serde::Deserializer;
    use serde::Serializer;

    pub(crate) fn serialize<S>(
        protocol: &Protocol<'static>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(protocol.to_string().as_str())
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Protocol<'static>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ProtocolVisitor;

        impl<'de> Visitor<'de> for ProtocolVisitor {
            type Value = Protocol<'static>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a protocol as a string")
            }

            fn visit_str<E>(self, string: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let mut iter = string.split('/');
                iter.next();
                let protocol: Protocol<'_> = Protocol::from_str_parts(iter).map_err(E::custom)?;
                Ok(protocol.acquire())
            }
        }

        deserializer.deserialize_str(ProtocolVisitor)
    }
}
