#[macro_use]
extern crate serde_derive;
pub mod data {
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Response {
        pub search: Vec<ImageResponse>
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ImageResponse {
        id: u32,
        mime_type: String,
        image: String,
        tags: String
    }

    pub fn get_images() -> Vec<ImageResponse> {
        let url = "https://derpibooru.org/search.json?q=faved_by:sapphie&perpage=50";

        let body = reqwest::get(url).unwrap().text().unwrap();

        let v: Response = serde_json::from_str(&body).unwrap();
        
        v.search

    }
}

pub mod bot {
}
    
