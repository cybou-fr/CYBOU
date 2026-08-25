// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Replaceable `LiteLLM` adapter behind Cybou's provider-neutral [`Worker`] interface.
//!
//! Provider credentials and routing stay in `LiteLLM`. The worker holds only its proxy master key
//! and mints a short-lived, model-scoped, budget-scoped virtual key for each completion.
//! Registration is valid only against a proxy with its database-backed budget reservation enabled
//! and pricing configured for every mapped model group; an unpriced route cannot prove the hard
//! pre-call reservation required by the lease.

use std::{collections::BTreeMap, num::NonZeroU64, time::Duration};

use cybou_model_brokerd::{
    ProviderChatOutput, ProviderChatRequest, UpstreamAttribution, Worker, WorkerFailed,
};
use cybou_protocol::model::{ModelManifest, ModelOutput, ModelRequest, SpendPolicy};
use reqwest::{StatusCode, blocking::Client};
use serde::{Deserialize, Serialize};

const VIRTUAL_KEY_DURATION: &str = "5m";

/// One Cybou capability class mapped to an operator-owned `LiteLLM` model group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiteLlmRoute {
    /// Capability class granted by the capsule lease.
    pub model_class: String,
    /// Alias from the `LiteLLM` `model_list`, not a provider model hardcoded into Cybou.
    pub model_group: String,
    /// Whether the operator has declared that this group bills nothing.
    ///
    /// Declared rather than inferred. Cybou cannot see a price list, and a worker that decided a
    /// group was free because the last completion happened to cost nothing would be treating an
    /// observation as a guarantee — which is precisely the distinction the provider catalogue was
    /// built to keep. Only a route carrying this may serve a `ZeroCostOnly` request.
    pub zero_cost: bool,
}

/// Invalid worker configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// The URL was not an absolute HTTP(S) endpoint without embedded credentials.
    InvalidBaseUrl,
    /// A class or group was empty or a class was mapped more than once.
    InvalidRoute,
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidBaseUrl => formatter.write_str("invalid LiteLLM base URL"),
            Self::InvalidRoute => formatter.write_str("invalid or duplicate LiteLLM route"),
        }
    }
}

impl core::error::Error for ConfigError {}

/// A `LiteLLM` proxy worker.
pub struct LiteLlmWorker {
    manifest: ModelManifest,
    base_url: String,
    master_key: String,
    routes: BTreeMap<String, (String, bool)>,
    /// Number of micro-US-dollars represented by one operator spend unit.
    microusd_per_unit: NonZeroU64,
    timeout_ms: u32,
    client: Client,
}

impl core::fmt::Debug for LiteLlmWorker {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("LiteLlmWorker")
            .field("manifest", &self.manifest)
            .field("base_url", &self.base_url)
            .field("routes", &self.routes)
            .field("microusd_per_unit", &self.microusd_per_unit)
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}

impl LiteLlmWorker {
    /// Build a worker without contacting the proxy.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] for an unsafe URL or ambiguous class mapping.
    pub fn new(
        manifest: ModelManifest,
        base_url: &str,
        master_key: String,
        routes: Vec<LiteLlmRoute>,
        microusd_per_unit: NonZeroU64,
        timeout_ms: u32,
    ) -> Result<Self, ConfigError> {
        let parsed = reqwest::Url::parse(base_url).map_err(|_| ConfigError::InvalidBaseUrl)?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(ConfigError::InvalidBaseUrl);
        }
        let mut mapped = BTreeMap::new();
        for route in routes {
            if route.model_class.trim().is_empty()
                || route.model_group.trim().is_empty()
                || mapped
                    .insert(route.model_class, (route.model_group, route.zero_cost))
                    .is_some()
            {
                return Err(ConfigError::InvalidRoute);
            }
        }
        let timeout_ms = timeout_ms.max(1);
        let client = Client::builder()
            .timeout(Duration::from_millis(u64::from(timeout_ms)))
            .build()
            .map_err(|_| ConfigError::InvalidBaseUrl)?;
        Ok(Self {
            manifest,
            base_url: base_url.trim_end_matches('/').to_owned(),
            master_key,
            routes: mapped,
            microusd_per_unit,
            timeout_ms,
            client,
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn virtual_key(
        &self,
        request: &ProviderChatRequest,
        group: &str,
    ) -> Result<String, WorkerFailed> {
        // Under a zero-cost policy the key carries no budget at all.
        //
        // Not a budget of zero: LiteLLM installations differ on whether that means nought or
        // unlimited, and a boundary that means opposite things on different deployments is not one.
        // The constraint is carried where it can be checked instead — the route had to be declared
        // free before it was reached, and a completion that bills anyway is refused below. The key
        // still names one model group and one concurrent request, so it is not a wider key than a
        // capped one; it is a key with a different thing bounding it.
        let max_budget = match request.spend {
            SpendPolicy::ZeroCostOnly => None,
            SpendPolicy::Capped(units) => {
                if units == 0 {
                    return Err(WorkerFailed::OutOfResources);
                }
                let max_microusd = units
                    .checked_mul(self.microusd_per_unit.get())
                    .ok_or(WorkerFailed::OutOfResources)?;
                Some(
                    format!(
                        "{}.{:06}",
                        max_microusd / 1_000_000,
                        max_microusd % 1_000_000
                    )
                    .parse::<serde_json::Number>()
                    .map_err(|_| WorkerFailed::OutOfResources)?,
                )
            }
        };
        let response = self
            .client
            .post(self.endpoint("/key/generate"))
            .bearer_auth(&self.master_key)
            .json(&KeyRequest {
                models: [group],
                duration: VIRTUAL_KEY_DURATION,
                max_budget,
                max_parallel_requests: 1,
                metadata: KeyMetadata {
                    cybou_request_id: request.request_id.to_string(),
                },
            })
            .send()
            .map_err(|error| self.transport_failure(&error))?;
        if !response.status().is_success() {
            return Err(status_failure(response.status(), "virtual key"));
        }
        response
            .json::<KeyResponse>()
            .map(|body| body.key)
            .map_err(|_| unusable("virtual-key response was malformed"))
    }

    fn chat(
        &self,
        request: &ProviderChatRequest,
        group: &str,
        virtual_key: &str,
    ) -> Result<ProviderChatOutput, WorkerFailed> {
        let response = self
            .client
            .post(self.endpoint("/v1/chat/completions"))
            .bearer_auth(virtual_key)
            .json(&ChatRequest {
                model: group,
                messages: request
                    .messages
                    .iter()
                    .map(|message| WireMessage {
                        role: &message.role,
                        content: &message.content,
                    })
                    .collect(),
                max_tokens: request.max_output_tokens,
                stream: false,
            })
            .send()
            .map_err(|error| self.transport_failure(&error))?;
        if !response.status().is_success() {
            return Err(status_failure(response.status(), "completion"));
        }

        let headers = response.headers();
        let cost = required_header(headers, "x-litellm-response-cost")?;
        let deployment_id = required_header(headers, "x-litellm-model-id")?;
        let returned_group = required_header(headers, "x-litellm-model-group")?;
        let call_id = required_header(headers, "x-litellm-call-id")?;
        if returned_group != group {
            return Err(unusable("proxy returned a different model group"));
        }
        let spend_units = decimal_usd_to_units(&cost, self.microusd_per_unit)?;
        let body = response
            .json::<ChatResponse>()
            .map_err(|_| unusable("completion response was malformed"))?;
        let content = body
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| unusable("completion response had no choice"))?
            .message
            .content;
        Ok(ProviderChatOutput {
            content,
            input_tokens: body.usage.prompt_tokens,
            output_tokens: body.usage.completion_tokens,
            spend_units,
            upstream: Some(UpstreamAttribution {
                model_group: returned_group,
                deployment_id,
                response_model: body.model,
                call_id,
            }),
        })
    }

    fn revoke(&self, virtual_key: &str) {
        // Best effort only: the key is already bounded by model, budget, concurrency and five
        // minutes. Cleanup cannot turn a provider answer into a retry and duplicate the spend.
        let _response = self
            .client
            .post(self.endpoint("/key/delete"))
            .bearer_auth(&self.master_key)
            .json(&DeleteKeys {
                keys: [virtual_key],
            })
            .send();
    }

    fn transport_failure(&self, error: &reqwest::Error) -> WorkerFailed {
        if error.is_timeout() {
            WorkerFailed::TimedOut {
                after_ms: self.timeout_ms,
            }
        } else if error.is_connect() {
            WorkerFailed::NotReady
        } else {
            unusable("proxy transport failed")
        }
    }
}

impl Worker for LiteLlmWorker {
    fn manifest(&self) -> &ModelManifest {
        &self.manifest
    }

    fn answer(&self, _request: &ModelRequest) -> Result<ModelOutput, WorkerFailed> {
        Err(WorkerFailed::UnsupportedSurface)
    }

    fn answer_chat(
        &self,
        request: &ProviderChatRequest,
    ) -> Result<ProviderChatOutput, WorkerFailed> {
        let (group, zero_cost) = self
            .routes
            .get(&request.model_class)
            .ok_or(WorkerFailed::UnsupportedSurface)?;
        // A zero-cost request may only be served by a route somebody declared costs nothing. Serving
        // it from a billable route "because it is probably cheap" would spend money against a
        // selection that said none, which is the one thing this policy exists to prevent.
        if matches!(request.spend, SpendPolicy::ZeroCostOnly) && !zero_cost {
            return Err(WorkerFailed::UnsupportedSurface);
        }
        let key = self.virtual_key(request, group)?;
        let answer = self.chat(request, group, &key);
        self.revoke(&key);

        let answer = answer?;
        // The promise was that this costs nothing. A charge means the route was not what it was
        // declared to be, and handing back an answer somebody has now been billed for — having asked
        // for none — would make the refusal cosmetic.
        if request.spend.broken_by(answer.spend_units) {
            return Err(WorkerFailed::OutOfResources);
        }
        Ok(answer)
    }
}

fn required_header(
    headers: &reqwest::header::HeaderMap,
    name: &str,
) -> Result<String, WorkerFailed> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| unusable("completion attribution header was missing"))
}

fn decimal_usd_to_units(value: &str, microusd_per_unit: NonZeroU64) -> Result<u64, WorkerFailed> {
    let (mantissa, exponent) = value
        .split_once(['e', 'E'])
        .map_or((value, 0_i32), |(mantissa, exponent)| {
            (mantissa, exponent.parse::<i32>().unwrap_or(i32::MIN))
        });
    if exponent == i32::MIN || mantissa.starts_with('-') || mantissa.starts_with('+') {
        return Err(unusable("completion cost was not a non-negative decimal"));
    }
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty()
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(unusable("completion cost was not a non-negative decimal"));
    }
    let digits = format!("{whole}{fraction}")
        .parse::<u128>()
        .map_err(|_| unusable("completion cost was too large"))?;
    let scale = exponent
        .checked_add(6)
        .and_then(|scale| scale.checked_sub(i32::try_from(fraction.len()).ok()?))
        .ok_or_else(|| unusable("completion cost was too precise"))?;
    let microusd = if scale >= 0 {
        digits
            .checked_mul(
                10_u128
                    .checked_pow(scale.unsigned_abs())
                    .ok_or_else(|| unusable("completion cost was too large"))?,
            )
            .ok_or_else(|| unusable("completion cost was too large"))?
    } else {
        let divisor = 10_u128
            .checked_pow(scale.unsigned_abs())
            .ok_or_else(|| unusable("completion cost was too precise"))?;
        digits.div_ceil(divisor)
    };
    let units = microusd.div_ceil(u128::from(microusd_per_unit.get()));
    u64::try_from(units).map_err(|_| unusable("completion cost was too large"))
}

fn status_failure(status: StatusCode, operation: &str) -> WorkerFailed {
    if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::PAYMENT_REQUIRED {
        WorkerFailed::OutOfResources
    } else if status == StatusCode::SERVICE_UNAVAILABLE || status == StatusCode::BAD_GATEWAY {
        WorkerFailed::NotReady
    } else {
        unusable(&format!("proxy refused {operation} with HTTP {status}"))
    }
}

fn unusable(detail: &str) -> WorkerFailed {
    WorkerFailed::Unusable {
        detail: detail.to_owned(),
    }
}

#[derive(Serialize)]
struct KeyRequest<'a> {
    models: [&'a str; 1],
    duration: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_budget: Option<serde_json::Number>,
    max_parallel_requests: u8,
    metadata: KeyMetadata,
}

#[derive(Serialize)]
struct KeyMetadata {
    cybou_request_id: String,
}

#[derive(Deserialize)]
struct KeyResponse {
    key: String,
}

#[derive(Serialize)]
struct DeleteKeys<'a> {
    keys: [&'a str; 1],
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<WireMessage<'a>>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize)]
struct WireMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: String,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_cost_is_rounded_up_and_never_down() {
        let unit = NonZeroU64::new(10).expect("non-zero");
        assert_eq!(decimal_usd_to_units("0", unit), Ok(0));
        assert_eq!(decimal_usd_to_units("0.000010", unit), Ok(1));
        assert_eq!(decimal_usd_to_units("0.0000101", unit), Ok(2));
        assert_eq!(decimal_usd_to_units("1e-5", unit), Ok(1));
    }
}
