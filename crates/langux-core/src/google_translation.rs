use std::{future::Future, time::Duration};

use reqwest::{Client, StatusCode, header::HeaderValue, redirect::Policy};
use serde_json::{Value, json};
use tokio::{runtime::Runtime, time::sleep};

use crate::{
    CancellationToken, LanguageCode, SourceLanguage, TranslationError, TranslationProvider,
    TranslationRequest, TranslationResult,
};

const TRANSLATE_ENDPOINT: &str = "https://translation.googleapis.com/language/translate/v2";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Translates text with Google Cloud Translation Basic v2.
///
/// The API key is held only in memory and sent in the `X-Goog-Api-Key` header.
/// Call [`TranslationProvider::translate`] from a worker thread so the
/// synchronous provider contract does not block the UI thread.
pub struct GoogleTranslationProvider {
    api_key: HeaderValue,
    client: Client,
    runtime: Runtime,
    endpoint: String,
}

impl GoogleTranslationProvider {
    /// Creates a provider that sends requests to Google's HTTPS endpoint.
    ///
    /// An empty key returns [`TranslationError::MissingCredential`]. A key
    /// containing invalid HTTP header bytes is rejected as unauthorized.
    pub fn new(api_key: impl AsRef<str>) -> Result<Self, TranslationError> {
        Self::build(api_key.as_ref(), TRANSLATE_ENDPOINT, true)
    }

    fn build(api_key: &str, endpoint: &str, require_https: bool) -> Result<Self, TranslationError> {
        if api_key.is_empty() {
            return Err(TranslationError::MissingCredential);
        }

        let mut api_key =
            HeaderValue::from_str(api_key).map_err(|_| TranslationError::UnauthorizedCredential)?;
        api_key.set_sensitive(true);

        let client = Client::builder()
            .https_only(require_https)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            // Never forward the API key to a redirect destination.
            .redirect(Policy::none())
            .build()
            .map_err(|_| TranslationError::ProviderFailure)?;

        let mut runtime_builder = tokio::runtime::Builder::new_multi_thread();
        runtime_builder
            .worker_threads(1)
            .thread_name("langux-translation")
            .enable_all();
        let runtime = runtime_builder
            .build()
            .map_err(|_| TranslationError::ProviderFailure)?;

        Ok(Self {
            api_key,
            client,
            runtime,
            endpoint: endpoint.to_owned(),
        })
    }

    async fn translate_request(
        &self,
        request: &TranslationRequest,
    ) -> Result<TranslationResult, TranslationError> {
        let mut body = json!({
            "q": request.text,
            "target": request.target_language.as_str(),
            "format": "text",
        });
        if let SourceLanguage::Specific(source) = &request.source_language {
            body["source"] = json!(source.as_str());
        }

        let response = self
            .client
            .post(&self.endpoint)
            .header("x-goog-api-key", self.api_key.clone())
            .json(&body)
            .send()
            .await
            .map_err(|_| TranslationError::NetworkFailure)?;
        let status = response.status();
        let response_body = response
            .bytes()
            .await
            .map_err(|_| TranslationError::NetworkFailure)?;

        if !status.is_success() {
            return Err(map_provider_error(status, &response_body));
        }

        parse_translation(&response_body)
    }
}

impl TranslationProvider for GoogleTranslationProvider {
    fn translate(
        &self,
        request: &TranslationRequest,
        cancellation: &CancellationToken,
    ) -> Result<TranslationResult, TranslationError> {
        if cancellation.is_cancelled() {
            return Err(TranslationError::Cancelled);
        }

        let outcome = self
            .runtime
            .block_on(cancelable(cancellation, self.translate_request(request)));

        if cancellation.is_cancelled() {
            Err(TranslationError::Cancelled)
        } else {
            outcome
        }
    }
}

async fn cancelable<F>(
    cancellation: &CancellationToken,
    operation: F,
) -> Result<TranslationResult, TranslationError>
where
    F: Future<Output = Result<TranslationResult, TranslationError>>,
{
    tokio::select! {
        biased;
        _ = wait_for_cancellation(cancellation) => Err(TranslationError::Cancelled),
        outcome = operation => outcome,
    }
}

async fn wait_for_cancellation(cancellation: &CancellationToken) {
    loop {
        if cancellation.is_cancelled() {
            return;
        }

        sleep(CANCELLATION_POLL_INTERVAL).await;
    }
}

fn parse_translation(body: &[u8]) -> Result<TranslationResult, TranslationError> {
    let response: Value =
        serde_json::from_slice(body).map_err(|_| TranslationError::MalformedResponse)?;
    let translation = response
        .get("data")
        .and_then(|data| data.get("translations"))
        .and_then(Value::as_array)
        .and_then(|translations| translations.first())
        .ok_or(TranslationError::MalformedResponse)?;
    let translated_text = translation
        .get("translatedText")
        .and_then(Value::as_str)
        .ok_or(TranslationError::MalformedResponse)?;
    let detected_source_language = match translation.get("detectedSourceLanguage") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let code = value.as_str().ok_or(TranslationError::MalformedResponse)?;
            Some(LanguageCode::new(code).map_err(|_| TranslationError::MalformedResponse)?)
        }
    };

    Ok(TranslationResult::new(
        translated_text,
        detected_source_language,
    ))
}

fn map_provider_error(status: StatusCode, body: &[u8]) -> TranslationError {
    let error = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|response| response.get("error").cloned())
        .unwrap_or(Value::Null);
    let status_name = error.get("status").and_then(Value::as_str).unwrap_or("");
    let reasons = error
        .get("errors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("reason").and_then(Value::as_str))
        .chain(
            error
                .get("details")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.get("reason").and_then(Value::as_str)),
        );
    let codes = std::iter::once(status_name).chain(reasons);

    let mut unauthorized = status == StatusCode::UNAUTHORIZED;
    let mut quota_exhausted = status == StatusCode::TOO_MANY_REQUESTS;
    for code in codes {
        match normalize_error_code(code).as_str() {
            "unauthenticated" | "keyinvalid" | "apikeyinvalid" | "invalidauthentication" => {
                unauthorized = true;
            }
            "quotaexceeded"
            | "ratelimitexceeded"
            | "userratelimitexceeded"
            | "dailylimitexceeded"
            | "resourceexhausted" => quota_exhausted = true,
            _ => {}
        }
    }

    if unauthorized {
        TranslationError::UnauthorizedCredential
    } else if quota_exhausted {
        TranslationError::QuotaOrRateLimit
    } else {
        TranslationError::ProviderFailure
    }
}

fn normalize_error_code(code: &str) -> String {
    code.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
#[path = "google_translation/tests.rs"]
mod tests;
