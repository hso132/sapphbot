use std::collections::HashSet;
use std::fs::read_to_string;
use std::time;

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub search: Vec<ImageResponse>,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageResponse {
    pub id: i64,
    pub mime_type: String,
    pub image: String,
    pub representations: Representations,
    pub tags: String,
    pub source_url: String,
    pub sha512_hash: String
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Representations {
    pub small: String,
    pub medium: String,
    pub large: String,
}

pub fn get_images() -> HashSet<ImageResponse> {
    let key = read_to_string("derpi_api_key.txt").unwrap();
    let url = format!("https://derpibooru.org/search.json?q=my:faves&key={}", key);

    let mut timeout = time::Duration::from_millis(1000);

    while timeout < time::Duration::from_secs(600) {
        if let Ok(mut resp) = reqwest::get(&url) {
            if resp.status().is_success() {
                if let Ok(body) = resp.text() {
                    let v: Response = serde_json::from_str(&body).unwrap();
                    return v.search.iter().cloned().collect();
                }
            } else {
                e!("Non-successful status received", resp);
            }
        }
        timeout = timeout * 2;
        std::thread::sleep(timeout);
    }

    HashSet::new()
}
