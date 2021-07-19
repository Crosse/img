use super::*;

/// List images.
pub fn list(filter: Option<&ImageFilter>) -> Result<Vec<Image>, Box<dyn Error>> {
    let url = match filter {
        Some(f) => Url::parse(&format!("{}?{}", JOYENT_IMGAPI_URL, f.to_string()))?,
        None => Url::parse(JOYENT_IMGAPI_URL)?,
    };

    println!("url: {}", url);
    let images: Vec<Image> = reqwest::blocking::get(url)?.json()?;
    Ok(images)
}

pub fn get(image_uuid: &str) -> Result<Image, Box<dyn Error>> {
    let _ = Uuid::parse_str(image_uuid)?;
    let base_url = Url::parse(JOYENT_IMGAPI_URL)?;
    let img_url = base_url.join(image_uuid)?;
    let img: Image = reqwest::blocking::get(img_url)?.json()?;
    Ok(img)
}
