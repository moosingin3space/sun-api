//! Durable Object for scheduled webhook storage and dispatch using Durable Object storage

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use worker::*;

fn method_to_string(method: &Method) -> String {
    match method {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Delete => "DELETE",
        Method::Patch => "PATCH",
        Method::Head => "HEAD",
        Method::Options => "OPTIONS",
        _ => "POST",
    }
    .to_string()
}

fn string_to_method(s: &str) -> Method {
    match s {
        "GET" => Method::Get,
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "DELETE" => Method::Delete,
        "PATCH" => Method::Patch,
        "HEAD" => Method::Head,
        "OPTIONS" => Method::Options,
        _ => Method::Post,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookRequest {
    pub url: String,
    #[serde(
        serialize_with = "serialize_method",
        deserialize_with = "deserialize_method"
    )]
    pub method: Method,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
    pub scheduled_at: jiff::Timestamp,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoredWebhook {
    pub id: String,
    pub url: String,
    #[serde(
        serialize_with = "serialize_method",
        deserialize_with = "deserialize_method"
    )]
    pub method: Method,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
    pub scheduled_at: jiff::Timestamp,
}

fn serialize_method<S>(method: &Method, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&method_to_string(method))
}

fn deserialize_method<'de, D>(deserializer: D) -> Result<Method, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(string_to_method(&s))
}

#[durable_object]
pub struct ScheduledWebhook {
    state: State,
}

impl DurableObject for ScheduledWebhook {
    fn new(state: State, _env: Env) -> Self {
        Self { state }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        match req.method() {
            Method::Post => {
                // Parse the incoming webhook configuration
                let webhook: WebhookRequest = match req.json().await {
                    Ok(w) => w,
                    Err(_) => return Response::error("Invalid JSON", 400),
                };

                // Generate unique ID
                let id = uuid::Uuid::new_v4().to_string();

                // Get the scheduled timestamp for alarm
                let scheduled_timestamp = webhook.scheduled_at;
                let alarm_time = scheduled_timestamp.as_millisecond();

                let stored_webhook = StoredWebhook {
                    id: id.clone(),
                    url: webhook.url,
                    method: webhook.method,
                    body: webhook.body,
                    headers: webhook.headers,
                    scheduled_at: scheduled_timestamp,
                };

                // Store in Durable Object storage
                let storage_key = format!("webhook:{}", id);
                self.state
                    .storage()
                    .put(&storage_key, &stored_webhook)
                    .await?;

                // Set an alarm for the scheduled time
                self.state.storage().set_alarm(alarm_time).await?;

                let response_json = serde_json::json!({
                    "id": id,
                    "message": "Webhook saved successfully",
                });

                Response::from_json(&response_json)
            }
            Method::Get => {
                // Retrieve all webhooks from Durable Object storage
                let storage = self.state.storage();
                let keys_map = storage.list().await?;

                let mut webhooks: Vec<StoredWebhook> = Vec::new();

                // Iterate over the Map keys
                let keys_iter = keys_map.keys();
                for key_result in keys_iter {
                    if let Ok(key_jsval) = key_result
                        && let Some(key_str) = key_jsval.as_string()
                        && let Some(webhook) = storage.get::<StoredWebhook>(&key_str).await?
                    {
                        webhooks.push(webhook);
                    }
                }

                Response::from_json(&webhooks)
            }
            _ => Response::error("Method Not Allowed", 405),
        }
    }

    async fn alarm(&self) -> Result<Response> {
        let storage = self.state.storage();

        // Get all webhooks from Durable Object storage
        let keys_map = storage.list().await?;

        let mut successful = 0;
        let mut failed = 0;

        // Iterate over the Map keys
        let keys_iter = keys_map.keys();
        for key_result in keys_iter {
            let key_str = match key_result {
                Ok(key_jsval) => match key_jsval.as_string() {
                    Some(s) => s,
                    None => {
                        failed += 1;
                        continue;
                    }
                },
                Err(_) => {
                    failed += 1;
                    continue;
                }
            };

            let webhook = match storage.get::<StoredWebhook>(&key_str).await? {
                Some(w) => w,
                None => {
                    failed += 1;
                    continue;
                }
            };

            // Create request
            let mut init = RequestInit::default();
            init.with_method(webhook.method);

            if let Some(body_str) = &webhook.body {
                init.with_body(Some(body_str.clone().into()));
            }

            let mut request = match Request::new_with_init(&webhook.url, &init) {
                Ok(r) => r,
                Err(_) => {
                    failed += 1;
                    continue;
                }
            };

            // Add headers to the request
            for (name, value) in &webhook.headers {
                if request.headers_mut()?.set(name, value).is_err() {
                    failed += 1;
                    continue;
                }
            }

            // Send the webhook request
            match Fetch::Request(request).send().await {
                Ok(_) => {
                    // Delete the webhook after successful dispatch
                    if storage.delete(&key_str).await.is_err() {
                        failed += 1;
                    } else {
                        successful += 1;
                    }
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        let response_json = serde_json::json!({
            "sent": successful,
            "failed": failed,
        });

        Response::from_json(&response_json)
    }
}
