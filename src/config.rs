use serde::{Deserialize, Serialize};
use std::{env, fs};
use webrtc::{
    ice,
    ice_transport::{ice_credential_type::RTCIceCredentialType, ice_server::RTCIceServer},
    Error,
};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_ice_servers")]
    pub ice_servers: Vec<IceServer>,
    #[serde(default)]
    pub auth: Auth,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Auth {
    pub accounts: Vec<Account>,
    pub tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub username: String,
    pub password: String,
}

fn default_listen() -> String {
    format!(
        "0.0.0.0:{}",
        env::var("PORT").unwrap_or(String::from("3000"))
    )
}

fn default_ice_servers() -> Vec<IceServer> {
    vec![IceServer {
        urls: vec!["stun:stun.l.google.com:19302".to_string()],
        username: "".to_string(),
        credential: "".to_string(),
        credential_type: "".to_string(),
    }]
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IceServer {
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub credential: String,
    #[serde(default)]
    pub credential_type: String,
}

// from https://github.com/webrtc-rs/webrtc/blob/71157ba2153a891a8cfd819f3cf1441a7a0808d8/webrtc/src/ice_transport/ice_server.rs
impl IceServer {
    pub(crate) fn parse_url(&self, url_str: &str) -> webrtc::error::Result<ice::url::Url> {
        Ok(ice::url::Url::parse_url(url_str)?)
    }

    pub(crate) fn validate(&self) -> webrtc::error::Result<()> {
        self.urls()?;
        Ok(())
    }

    pub(crate) fn urls(&self) -> webrtc::error::Result<Vec<ice::url::Url>> {
        let mut urls = vec![];

        for url_str in &self.urls {
            let mut url = self.parse_url(url_str)?;
            if url.scheme == ice::url::SchemeType::Turn || url.scheme == ice::url::SchemeType::Turns
            {
                // https://www.w3.org/TR/webrtc/#set-the-configuration (step #11.3.2)
                if self.username.is_empty() || self.credential.is_empty() {
                    return Err(Error::ErrNoTurnCredentials);
                }
                url.username = self.username.clone();

                match self.credential_type.as_str().into() {
                    RTCIceCredentialType::Password => {
                        // https://www.w3.org/TR/webrtc/#set-the-configuration (step #11.3.3)
                        url.password = self.credential.clone();
                    }
                    RTCIceCredentialType::Oauth => {
                        // https://www.w3.org/TR/webrtc/#set-the-configuration (step #11.3.4)
                        /*if _, ok: = s.Credential.(OAuthCredential); !ok {
                                return nil,
                                &rtcerr.InvalidAccessError{Err: ErrTurnCredentials
                            }
                        }*/
                    }
                    _ => return Err(Error::ErrTurnCredentials),
                };
            }

            urls.push(url);
        }

        Ok(urls)
    }
}

impl From<IceServer> for RTCIceServer {
    fn from(val: IceServer) -> Self {
        RTCIceServer {
            urls: val.urls,
            username: val.username,
            credential: val.credential,
            credential_type: val.credential_type.as_str().into(),
        }
    }
}

impl Config {
    pub(crate) fn parse() -> Self {
        let mut result = fs::read_to_string("config.toml");
        if result.is_err() {
            result = fs::read_to_string("/etc/live777/config.toml");
        }
        if let Ok(cfg) = result {
            let cfg: Self = toml::from_str(cfg.as_str()).expect("config parse error");
            match cfg.validate() {
                Ok(_) => cfg,
                Err(err) => panic!("config validate [{}]", err),
            }
        } else {
            Config {
                ice_servers: default_ice_servers(),
                listen: default_listen(),
                auth: Default::default(),
            }
        }
    }

    fn validate(&self) -> anyhow::Result<()> {
        for ice_server in self.ice_servers.iter() {
            ice_server
                .validate()
                .map_err(|e| anyhow::anyhow!(format!("ice_server error : {}", e)))?;
        }
        Ok(())
    }
}
