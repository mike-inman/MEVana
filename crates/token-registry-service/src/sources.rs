use crate::model::PartialToken;
use anyhow::Result;
use base64::Engine;
use chrono::Utc;
use futures::future::BoxFuture;

pub trait TokenSource: Send + Sync {
    fn fetch(&self, mint: &str) -> BoxFuture<'static, Result<Option<PartialToken>>>;
    fn id(&self) -> &str;
    fn as_any(&self) -> &dyn std::any::Any;
}

pub struct StaticOverrideSource {
    pub map: std::collections::HashMap<
        String,
        (Option<String>, Option<String>, Option<u8>, Option<String>),
    >, // mint -> (symbol,name,decimals,logo_url)
}

impl TokenSource for StaticOverrideSource {
    fn fetch(&self, mint: &str) -> BoxFuture<'static, Result<Option<PartialToken>>> {
        let opt = self.map.get(mint).cloned();
        let source_id = self.id().to_string();
        let owned_mint = mint.to_string();
        Box::pin(async move {
            if let Some((sym, name, dec, logo)) = opt {
                Ok(Some(PartialToken {
                    mint: owned_mint,
                    symbol: sym,
                    name,
                    decimals: dec,
                    logo_url: logo,
                    tags: None,
                    is_stable: None,
                    is_wrapped: None,
                    is_lp: None,
                    risk_flags: None,
                    source_id,
                    fetched_at: Utc::now(),
                    weight: 10.0,
                }))
            } else {
                Ok(None)
            }
        })
    }
    fn id(&self) -> &str {
        "static_override"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct OnChainMintSource {
    pub rpc_url: String,
    pub client: reqwest::Client,
}

impl TokenSource for OnChainMintSource {
    fn fetch(&self, mint: &str) -> BoxFuture<'static, Result<Option<PartialToken>>> {
        let client = self.client.clone();
        let rpc = self.rpc_url.clone();
        let m = mint.to_string();
        let source_id = self.id().to_string();
        Box::pin(async move {
            let payload = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"getAccountInfo","params":[m.clone(),{"encoding":"base64"}]});
            let resp = client.post(&rpc).json(&payload).send().await?;
            if !resp.status().is_success() {
                return Ok(None);
            }
            let v: serde_json::Value = resp.json().await?;
            let data_b64 = v["result"]["value"]["data"][0].as_str();
            let decimals = data_b64
                .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
                .and_then(|bytes| bytes.get(44).copied());
            Ok(Some(PartialToken {
                mint: m,
                symbol: None,
                name: None,
                decimals,
                logo_url: None,
                tags: None,
                is_stable: None,
                is_wrapped: None,
                is_lp: None,
                risk_flags: None,
                source_id,
                fetched_at: Utc::now(),
                weight: 5.0,
            }))
        })
    }
    fn id(&self) -> &str {
        "onchain_mint"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
