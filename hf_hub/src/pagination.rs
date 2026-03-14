use std::collections::VecDeque;
use futures::stream::{self, Stream};
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use url::Url;
use crate::client::HfApi;
use crate::error::{HfError, Result};

struct PaginationState {
    buffer: VecDeque<serde_json::Value>,
    next_url: Option<Url>,
    is_first_page: bool,
    done: bool,
}

impl HfApi {
    /// Create a paginated stream from an initial URL and query params.
    /// Query params are only sent on the first request; subsequent pages
    /// use the full URL from the Link header.
    pub(crate) fn paginate<T: DeserializeOwned + 'static>(
        &self,
        initial_url: Url,
        params: Vec<(String, String)>,
    ) -> impl Stream<Item = Result<T>> + '_ {
        let state = PaginationState {
            buffer: VecDeque::new(),
            next_url: Some(initial_url),
            is_first_page: true,
            done: false,
        };

        stream::try_unfold(state, move |mut state| {
            let params = params.clone();
            async move {
                // Drain buffer first
                if let Some(raw) = state.buffer.pop_front() {
                    let item: T = serde_json::from_value(raw)?;
                    return Ok(Some((item, state)));
                }

                if state.done {
                    return Ok(None);
                }

                let url = match state.next_url.take() {
                    Some(u) => u,
                    None => {
                        state.done = true;
                        return Ok(None);
                    }
                };

                let mut request = self.inner.client.get(url.clone())
                    .headers(self.auth_headers());
                if state.is_first_page {
                    request = request.query(&params);
                    state.is_first_page = false;
                }

                let response = request.send().await?;

                if !response.status().is_success() {
                    let status = response.status();
                    let resp_url = response.url().to_string();
                    let body = response.text().await.unwrap_or_default();
                    return Err(HfError::Http {
                        status,
                        url: resp_url,
                        body,
                    });
                }

                state.next_url = parse_link_header_next(response.headers());
                if state.next_url.is_none() {
                    state.done = true;
                }

                let items: Vec<serde_json::Value> = response.json().await?;
                state.buffer = VecDeque::from(items);

                match state.buffer.pop_front() {
                    Some(raw) => {
                        let item: T = serde_json::from_value(raw)?;
                        Ok(Some((item, state)))
                    }
                    None => Ok(None),
                }
            }
        })
    }
}

/// Parse the `Link` header for a `rel="next"` URL.
/// Format: `<https://huggingface.co/api/models?p=1>; rel="next"`
fn parse_link_header_next(headers: &HeaderMap) -> Option<Url> {
    let link_header = headers.get("link")?.to_str().ok()?;

    for part in link_header.split(',') {
        let part = part.trim();
        // Check if this segment has rel="next"
        if !part.contains("rel=\"next\"") {
            continue;
        }
        // Extract URL between < and >
        let start = part.find('<')? + 1;
        let end = part.find('>')?;
        let url_str = &part[start..end];
        return Url::parse(url_str).ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn test_parse_link_header_next() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "link",
            HeaderValue::from_static(
                r#"<https://huggingface.co/api/models?p=1>; rel="next""#,
            ),
        );
        let url = parse_link_header_next(&headers).unwrap();
        assert_eq!(url.as_str(), "https://huggingface.co/api/models?p=1");
    }

    #[test]
    fn test_parse_link_header_no_next() {
        let headers = HeaderMap::new();
        assert!(parse_link_header_next(&headers).is_none());
    }

    #[test]
    fn test_parse_link_header_multiple_rels() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "link",
            HeaderValue::from_static(
                r#"<https://example.com/prev>; rel="prev", <https://example.com/next>; rel="next""#,
            ),
        );
        let url = parse_link_header_next(&headers).unwrap();
        assert_eq!(url.as_str(), "https://example.com/next");
    }
}
