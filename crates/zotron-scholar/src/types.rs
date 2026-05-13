use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub title: String,
    pub authors: Vec<Author>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub abstract_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arxiv_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl Paper {
    /// Convert to Zotero JSON item format for import via `zotron push`.
    pub fn to_zotero_json(&self) -> serde_json::Value {
        let creators: Vec<serde_json::Value> = self
            .authors
            .iter()
            .map(|a| {
                let parts: Vec<&str> = a.name.rsplitn(2, ' ').collect();
                let (last, first) = if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    (a.name.clone(), String::new())
                };
                serde_json::json!({
                    "creatorType": "author",
                    "firstName": first,
                    "lastName": last,
                })
            })
            .collect();

        let mut item = serde_json::json!({
            "itemType": "journalArticle",
            "title": self.title,
            "creators": creators,
        });

        let obj = item.as_object_mut().unwrap();

        if let Some(ref doi) = self.doi {
            obj.insert("DOI".into(), serde_json::Value::String(doi.clone()));
        }
        if let Some(ref abs) = self.abstract_text {
            obj.insert(
                "abstractNote".into(),
                serde_json::Value::String(abs.clone()),
            );
        }
        if let Some(ref date) = self.date {
            obj.insert("date".into(), serde_json::Value::String(date.clone()));
        }
        if let Some(ref pub_name) = self.publication {
            obj.insert(
                "publicationTitle".into(),
                serde_json::Value::String(pub_name.clone()),
            );
        }
        if let Some(ref vol) = self.volume {
            obj.insert("volume".into(), serde_json::Value::String(vol.clone()));
        }
        if let Some(ref pages) = self.pages {
            obj.insert("pages".into(), serde_json::Value::String(pages.clone()));
        }
        if let Some(ref url) = self.url {
            obj.insert("url".into(), serde_json::Value::String(url.clone()));
        }

        item
    }
}
