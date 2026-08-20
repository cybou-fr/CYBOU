// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Strict compatibility layer for the versioned C++/Qt cognitive fabric.

use std::io::Cursor;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

pub mod event_client;
pub mod rpc;
#[cfg(target_os = "linux")]
pub mod zbus_rpc;

/// Version emitted by the existing `FabricCodec` implementation.
pub const FABRIC_VERSION: u16 = 1;

/// Stable address of one process-owned D-Bus contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BusEndpoint {
    /// Well-known bus name.
    pub service: &'static str,
    /// Exported object path.
    pub object_path: &'static str,
    /// Versioned interface name.
    pub interface: &'static str,
    /// Activatable systemd unit.
    pub systemd_unit: &'static str,
}

macro_rules! endpoint {
    ($name:ident, $owner:literal, $unit:literal) => {
        #[doc = concat!("Stable endpoint for `", $owner, "1`.")]
        pub const $name: BusEndpoint = BusEndpoint {
            service: concat!("org.cybou.Mind.", $owner, "1"),
            object_path: concat!("/org/cybou/Mind/", $owner, "1"),
            interface: concat!("org.cybou.Mind.", $owner, "1"),
            systemd_unit: $unit,
        };
    };
}

endpoint!(EVENT, "Event", "cybou-eventd.service");
endpoint!(PERCEPTION, "Perception", "cybou-perceptiond.service");
endpoint!(CONTEXT, "Context", "cybou-contextd.service");
endpoint!(EPISTEMIC, "Epistemic", "cybou-epistemicd.service");
endpoint!(HEALTH, "Health", "cybou-healthd.service");
endpoint!(IDENTITY, "Identity", "cybou-identityd.service");
endpoint!(INTENTION, "Intention", "cybou-intentiond.service");
endpoint!(PREDICTOR, "Predictor", "cybou-predictord.service");
endpoint!(SELF, "Self", "cybou-selfd.service");
endpoint!(WORKSPACE, "Workspace", "cybou-workspaced.service");
endpoint!(PRESENCE, "Presence", "cybou-presenced.service");
endpoint!(LIFECYCLE, "Lifecycle", "cybou-lifecycled.service");
endpoint!(MEANING, "Meaning", "cybou-meaningd.service");

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireEnvelope<T> {
    version: u16,
    value: T,
}

/// Strict cognitive-fabric codec failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum FabricError {
    /// CBOR is malformed, incomplete, has unknown fields, or has trailing bytes.
    #[error("invalid cognitive fabric CBOR")]
    InvalidCbor,
    /// The peer uses a fabric version this binary does not implement.
    #[error("unsupported cognitive fabric version {0}")]
    UnsupportedVersion(u16),
}

/// Encode one value in the byte-compatible fabric v1 envelope.
///
/// # Errors
///
/// Returns [`FabricError::InvalidCbor`] when the value cannot be serialized.
pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, FabricError> {
    let envelope = WireEnvelope {
        version: FABRIC_VERSION,
        value,
    };
    let mut encoded = Vec::new();
    ciborium::into_writer(&envelope, &mut encoded).map_err(|_| FabricError::InvalidCbor)?;
    Ok(encoded)
}

/// Decode one strict fabric v1 envelope without accepting trailing data.
///
/// # Errors
///
/// Returns a typed error for malformed CBOR or an unsupported version.
pub fn decode<T: DeserializeOwned>(encoded: &[u8]) -> Result<T, FabricError> {
    let mut reader = Cursor::new(encoded);
    let envelope: WireEnvelope<T> =
        ciborium::from_reader(&mut reader).map_err(|_| FabricError::InvalidCbor)?;
    if reader.position() != encoded.len() as u64 {
        return Err(FabricError::InvalidCbor);
    }
    if envelope.version != FABRIC_VERSION {
        return Err(FabricError::UnsupportedVersion(envelope.version));
    }
    Ok(envelope.value)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{FabricError, PRESENCE, decode, encode};

    const QT_MAP_HEX: &str = include_str!("../../../fixtures/fabric/v1/map.hex");
    const QT_LIST_HEX: &str = include_str!("../../../fixtures/fabric/v1/list.hex");

    fn hex_bytes(value: &str) -> Vec<u8> {
        value
            .trim()
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex"), 16)
                    .expect("valid hex byte")
            })
            .collect()
    }

    #[test]
    fn rust_map_is_byte_identical_to_qt_fabric_codec() {
        let value = BTreeMap::from([("capability", "mind.identity.read"), ("state", "available")]);
        assert_eq!(encode(&value).expect("encode map"), hex_bytes(QT_MAP_HEX));
        assert_eq!(
            decode::<BTreeMap<String, String>>(&hex_bytes(QT_MAP_HEX)).expect("decode map")["state"],
            "available"
        );
    }

    #[test]
    fn rust_list_is_byte_identical_to_qt_fabric_codec() {
        let value = ["available", "unknown", "unavailable"];
        assert_eq!(encode(&value).expect("encode list"), hex_bytes(QT_LIST_HEX));
        assert_eq!(
            decode::<Vec<String>>(&hex_bytes(QT_LIST_HEX))
                .expect("decode list")
                .len(),
            3
        );
    }

    #[test]
    fn future_version_and_trailing_bytes_fail_closed() {
        let mut future = hex_bytes(QT_LIST_HEX);
        future[9] = 2;
        assert_eq!(
            decode::<Vec<String>>(&future),
            Err(FabricError::UnsupportedVersion(2))
        );

        let mut trailing = hex_bytes(QT_LIST_HEX);
        trailing.push(0);
        assert_eq!(
            decode::<Vec<String>>(&trailing),
            Err(FabricError::InvalidCbor)
        );
    }

    #[test]
    fn presence_endpoint_matches_the_frozen_cpp_registry() {
        assert_eq!(PRESENCE.service, "org.cybou.Mind.Presence1");
        assert_eq!(PRESENCE.object_path, "/org/cybou/Mind/Presence1");
        assert_eq!(PRESENCE.systemd_unit, "cybou-presenced.service");
    }
}
