//! Adapter that lets `aws-sdk-s3` send its (already signed) requests through
//! `frankmd_net::TrustedClient`, so S3 shares the app's single HTTP stack
//! (rustls, no proxy, finite timeouts) instead of bundling its own.

use aws_smithy_runtime_api::client::http::{
    HttpClient, HttpConnector, HttpConnectorFuture, HttpConnectorSettings, SharedHttpConnector,
};
use aws_smithy_runtime_api::client::orchestrator::{HttpRequest, HttpResponse};
use aws_smithy_runtime_api::client::result::ConnectorError;
use aws_smithy_runtime_api::client::runtime_components::RuntimeComponents;
use aws_smithy_runtime_api::http::StatusCode;
use aws_smithy_types::body::SdkBody;

/// `HttpClient` backed by the trusted reqwest client.
#[derive(Debug, Clone)]
pub(crate) struct TrustedHttpClient {
    http: frankmd_net::TrustedClient,
}

impl TrustedHttpClient {
    /// Wrap a trusted client.
    pub(crate) fn new(http: frankmd_net::TrustedClient) -> Self {
        Self { http }
    }
}

impl HttpClient for TrustedHttpClient {
    fn http_connector(
        &self,
        _settings: &HttpConnectorSettings,
        _components: &RuntimeComponents,
    ) -> SharedHttpConnector {
        SharedHttpConnector::new(self.clone())
    }
}

impl HttpConnector for TrustedHttpClient {
    fn call(&self, request: HttpRequest) -> HttpConnectorFuture {
        let http = self.http.clone();
        HttpConnectorFuture::new(async move { send(&http, request).await })
    }
}

/// Perform one request.
async fn send(
    http: &frankmd_net::TrustedClient,
    request: HttpRequest,
) -> Result<HttpResponse, ConnectorError> {
    let method = reqwest::Method::from_bytes(request.method().as_bytes())
        .map_err(|e| ConnectorError::user(e.into()))?;
    let Some(body) = request.body().bytes() else {
        // Every S3 call FrankMD makes has an in-memory body.
        return Err(ConnectorError::user(
            "streaming request bodies are not supported".into(),
        ));
    };
    let mut builder = http
        .reqwest()
        .request(method, request.uri())
        .body(body.to_vec());
    for (name, value) in request.headers() {
        builder = builder.header(name, value);
    }
    let response = builder.send().await.map_err(|e| {
        if e.is_timeout() {
            ConnectorError::timeout(e.without_url().into())
        } else {
            ConnectorError::io(e.without_url().into())
        }
    })?;
    let status = StatusCode::try_from(response.status().as_u16())
        .map_err(|e| ConnectorError::other(e.into(), None))?;
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .filter_map(|(k, v)| Some((k.as_str().to_owned(), v.to_str().ok()?.to_owned())))
        .collect();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| ConnectorError::io(e.without_url().into()))?;
    let mut out = HttpResponse::new(status, SdkBody::from(bytes.to_vec()));
    for (name, value) in headers {
        out.headers_mut().append(name, value);
    }
    Ok(out)
}
