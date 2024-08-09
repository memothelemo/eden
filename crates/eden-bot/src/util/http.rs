use eden_utils::error::{exts::*, Result};
use futures::{FutureExt, TryFutureExt};
use serde::de::DeserializeOwned;
use std::future::IntoFuture;
use std::result::Result as StdResult;
use tracing::trace;
use twilight_http::request::TryIntoRequest;
use twilight_http::response::marker::ListBody;

use crate::errors::tags::RequestHttpTag;
use crate::errors::RequestHttpError;

/// Simplifies fetching request and transforming [`twilight_http::Error`]
/// into [Eden's error type](eden_utils::Error).
///
/// Unlike [`request_for_model`], it fetches data but it is used for requests
/// that expect to have a list of entries.
///
/// This is to make the entire code cleaner that way instead of doing this:
/// ```no_run
/// // And yes, this code is a mess to look at.
/// bot.http.create_message(channel.id)
///     .content("Hello world!")
///     .expect("Oops!")
///     .map(|v| v.into_eden_error().anonymize_error())
///     .and_then(|v| v.model().map(|v| v.into_typed_error().anonymize_error()))
///     .await
/// ```
///
/// **Usage**:
/// ```no_run
/// let request = bot.http.guild_members(guild.id)
///     .limit(500)
///     .unwrap();
///
/// let response = request_for_list(&bot.http, request).await?;
/// ```
#[tracing::instrument(skip_all, fields(
    method = tracing::field::Empty,
    path = tracing::field::Empty,
))]
pub async fn request_for_list<
    M: DeserializeOwned + Unpin,
    T: IntoFuture<Output = StdResult<twilight_http::Response<ListBody<M>>, twilight_http::Error>>
        + TryIntoRequest,
>(
    client: &twilight_http::Client,
    request: T,
) -> Result<Vec<M>, RequestHttpError> {
    let request = request.try_into_request().unwrap();
    let tag = RequestHttpTag::new(request.method(), request.path());

    let span = tracing::Span::current();
    if !span.is_disabled() {
        let method = request.method().to_http();
        let path = request.path().to_string();
        span.record("request.method", tracing::field::display(&method));
        span.record("request.path", tracing::field::display(&path));
    }

    trace!("fetching request for list");
    let list = client
        .request::<Vec<M>>(request)
        .map(|v| v.into_eden_error().anonymize_error())
        .and_then(|v| v.model().map(|v| v.into_typed_error().anonymize_error()))
        .await
        .change_context(RequestHttpError)
        .attach(tag)?;

    Ok(list)
}

/// Simplifies getting the response model data from a request data
/// and transforming its error into [Eden's error type](eden_utils::Error).
#[tracing::instrument(skip_all, fields(
    method = tracing::field::Empty,
    path = tracing::field::Empty,
))]
pub async fn request_for_model<
    M: DeserializeOwned + Unpin,
    T: IntoFuture<Output = StdResult<twilight_http::Response<M>, twilight_http::Error>>
        + TryIntoRequest,
>(
    client: &twilight_http::Client,
    request: T,
) -> Result<M, RequestHttpError> {
    let request = request.try_into_request().unwrap();
    let tag = RequestHttpTag::new(request.method(), request.path());

    let span = tracing::Span::current();
    if !span.is_disabled() {
        let method = request.method().to_http();
        let path = request.path().to_string();
        span.record("request.method", tracing::field::display(&method));
        span.record("request.path", tracing::field::display(&path));
    }

    trace!("fetching request for model");
    let response = client
        .request::<M>(request)
        .map(|v| v.into_eden_error().anonymize_error())
        .and_then(|v| v.model().map(|v| v.into_typed_error().anonymize_error()))
        .await
        .change_context(RequestHttpError)
        .attach(tag)?;

    Ok(response)
}
