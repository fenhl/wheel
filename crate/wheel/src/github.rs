#![allow(missing_docs)] //TODO

use {
    std::{
        future::Future,
        time::Duration,
    },
    jiff::Timestamp,
    reqwest::{
        Body,
        Client,
        StatusCode,
    },
    semver::Version,
    serde::Deserialize,
    serde_json::json,
    tokio::time::sleep,
    url::Url,
    crate::traits::{
        IsNetworkError,
        ReqwestResponseExt as _,
        ResponseStatusError,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] GitHubAuth(#[from] github_app_auth::AuthError),
    #[error(transparent)] HeaderToStr(#[from] reqwest::header::ToStrError),
    #[error(transparent)] Jiff(#[from] jiff::Error),
    #[error(transparent)] Json(#[from] crate::traits::ResponseJsonError),
    #[error(transparent)] ParseInt(#[from] std::num::ParseIntError),
    #[error(transparent)] Reqwest(#[from] reqwest::Error),
    #[error(transparent)] ResponseStatus(#[from] ResponseStatusError),
    #[error("missing x-ratelimit-reset header in GitHub error response")]
    MissingRateLimitResetHeader,
    #[error("attempted to send GitHub API request with streamed body")]
    UncloneableRequest,
}

impl IsNetworkError for Error {
    fn is_network_error(&self) -> bool {
        match self {
            Self::GitHubAuth(_) => false,
            Self::HeaderToStr(_) => false,
            Self::Jiff(_) => false,
            Self::Json(e) => e.is_network_error(),
            Self::ParseInt(_) => false,
            Self::Reqwest(e) => e.is_network_error(),
            Self::ResponseStatus(e) => e.is_network_error(),
            Self::MissingRateLimitResetHeader => false,
            Self::UncloneableRequest => false,
        }
    }
}

/// Adds a `send_github` method which automatically handles the GitHub REST API's rate limits.
pub trait RequestBuilderExt {
    /// Like `send` but automatically handles the GitHub REST API's rate limits.
    async fn send_github(self, verbose: bool) -> Result<reqwest::Response, Error>;
}

impl RequestBuilderExt for reqwest::RequestBuilder {
    /// Like `send` but automatically handles the GitHub REST API's rate limits.
    ///
    /// # Errors
    ///
    /// In addition to errors from `send` and errors parsing the rate limiting headers, this method will error if the request has a streaming body.
    async fn send_github(self, verbose: bool) -> Result<reqwest::Response, Error> {
        let mut exponential_backoff = Duration::from_secs(60);
        loop {
            match self.try_clone().ok_or(Error::UncloneableRequest)?.send().await?.detailed_error_for_status().await {
                Ok(response) => break Ok(response),
                Err(ResponseStatusError { inner, headers, text }) if inner.status().is_some_and(|status| matches!(status, StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS)) => {
                    if let Some(retry_after) = headers.get(reqwest::header::RETRY_AFTER) {
                        let delta = Duration::from_secs(retry_after.to_str()?.parse()?);
                        if verbose {
                            println!("{} Received retry_after, sleeping for {delta:?}", Timestamp::now().strftime("%F %T"));
                        }
                        sleep(delta).await;
                    } else if headers.get("x-ratelimit-remaining").is_some_and(|x_ratelimit_remaining| x_ratelimit_remaining == "0") {
                        let now = Timestamp::now();
                        let until = Timestamp::from_second(headers.get("x-ratelimit-reset").ok_or(Error::MissingRateLimitResetHeader)?.to_str()?.parse()?)?;
                        if let Ok(delta) = Duration::try_from(until - now) {
                            if verbose {
                                println!("{} Received x-ratelimit-remaining, sleeping for {delta:?}", Timestamp::now().strftime("%F %T"));
                            }
                            sleep(delta).await;
                        }
                    } else if exponential_backoff >= Duration::from_secs(60 * 60) {
                        break Err(Error::ResponseStatus(ResponseStatusError { inner, headers, text }))
                    } else {
                        if verbose {
                            println!("{} Received unspecific rate limit error, sleeping for {exponential_backoff:?}", Timestamp::now().strftime("%F %T"));
                        }
                        sleep(exponential_backoff).await;
                        exponential_backoff *= 2;
                    }
                }
                Err(e) => break Err(e.into()),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    number: u64,
    pub labels: Vec<Label>,
}

impl Issue {
    pub async fn set_labels(&self, client: &Client, token: &mut github_app_auth::InstallationAccessToken, repo: &Repo, labels: &[String]) -> Result<(), Error> {
        client.patch(&format!("https://api.github.com/repos/{}/{}/issues/{}", repo.user, repo.name, self.number))
            .headers(token.header().await?)
            .json(&json!({
                "labels": labels,
            }))
            .send_github(false).await?
            .detailed_error_for_status().await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub assets: Vec<ReleaseAsset>,
    id: u64,
    pub tag_name: String,
    upload_url: String,
}

impl Release {
    pub fn version(&self) -> Result<Version, semver::Error> {
        self.tag_name[1..].parse()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: Url,
}

/// A GitHub repository. Provides API methods.
#[derive(Clone)]
pub struct Repo {
    /// The GitHub user or organization who owns this repo.
    user: String,
    /// The name of the repo.
    name: String,
}

impl Repo {
    pub fn new(user: impl ToString, name: impl ToString) -> Self {
        Self {
            user: user.to_string(),
            name: name.to_string(),
        }
    }

    pub async fn issues_with_label(&self, client: &Client, token: &mut github_app_auth::InstallationAccessToken, label: &str) -> Result<Vec<Issue>, Error> {
        Ok(client.get(&format!("https://api.github.com/repos/{}/{}/issues", self.user, self.name))
            .headers(token.header().await?)
            .query(&[
                ("state", "all"),
                ("labels", label),
            ])
            .send_github(false).await?
            .detailed_error_for_status().await?
            .json_with_text_in_error().await?)
    }

    pub async fn latest_release(&self, client: &Client) -> Result<Option<Release>, Error> {
        let response = client.get(&format!("https://api.github.com/repos/{}/{}/releases/latest", self.user, self.name))
            .send_github(false).await?;
        if response.status() == StatusCode::NOT_FOUND { return Ok(None) } // no releases yet
        Ok(Some(
            response.detailed_error_for_status().await?
                .json_with_text_in_error::<Release>().await?
        ))
    }

    pub fn latest_release_sync(&self, client: &reqwest::blocking::Client) -> Result<Option<Release>, Error> {
        let response = client.get(&format!("https://api.github.com/repos/{}/{}/releases/latest", self.user, self.name))
            .send()?;
        if response.status() == StatusCode::NOT_FOUND { return Ok(None) } // no releases yet
        Ok(Some(
            response.error_for_status()?
                .json::<Release>()?
        ))
    }

    pub async fn release_by_tag(&self, client: &Client, tag: &str) -> Result<Option<Release>, Error> {
        let response = client.get(&format!("https://api.github.com/repos/{}/{}/releases/tags/{tag}", self.user, self.name))
            .send_github(false).await?;
        if response.status() == StatusCode::NOT_FOUND { return Ok(None) } // no release with this tag
        Ok(Some(
            response.detailed_error_for_status().await?
                .json_with_text_in_error::<Release>().await?
        ))
    }

    /// Creates a draft release, which can be published using `Repo::publish_release`.
    pub async fn create_release(&self, client: &Client, name: String, tag_name: String, body: String) -> Result<Release, Error> {
        Ok(
            client.post(&format!("https://api.github.com/repos/{}/{}/releases", self.user, self.name))
                .json(&json!({
                    "body": body,
                    "draft": true,
                    "name": name,
                    "tag_name": tag_name,
                }))
                .send_github(false).await?
                .detailed_error_for_status().await?
                .json_with_text_in_error::<Release>().await?
        )
    }

    pub async fn publish_release(&self, client: &Client, release: Release) -> Result<Release, Error> {
        Ok(
            client.patch(&format!("https://api.github.com/repos/{}/{}/releases/{}", self.user, self.name, release.id))
                .json(&json!({"draft": false}))
                .send_github(false).await?
                .detailed_error_for_status().await?
                .json_with_text_in_error::<Release>().await?
        )
    }

    pub fn release_attach<'a>(&self, client: &'a Client, release: &'a Release, name: &'a str, content_type: &'static str, body: impl Into<Body> + 'a) -> impl Future<Output = Result<(), Error>> + 'a {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::CONTENT_TYPE, reqwest::header::HeaderValue::from_static(content_type));
        async move {
            client.post(&release.upload_url.replace("{?name,label}", ""))
                .query(&[("name", name)])
                .headers(headers)
                .body(body)
                .send_github(false).await?
                .detailed_error_for_status().await?;
            Ok(())
        }
    }
}
