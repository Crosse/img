use std::env;
use std::error::Error;
use std::str::FromStr;

use uuid::Uuid;

use img::imgapi;

fn main() {
    if let Err(e) = process() {
        eprintln!("error: {}", e);
    }
}

fn process() -> Result<(), Box<dyn Error>> {
    let mut filter = imgapi::ImageFilter::default();
    for arg in env::args() {
        if let Some((k, v)) = arg.split_once("=") {
            let v = v.to_string();
            match k {
                "account" => {
                    filter.account =
                        Some(Uuid::parse_str(&v).map_err(|_| "account must be a valid UUID")?)
                }
                "channel" => filter.channel = Some(v),
                "inclAdminFields" => {
                    filter.include_admin_fields = Some(
                        bool::from_str(&v)
                            .map_err(|_| "inclAdminFields must be either true or false")?,
                    )
                }
                "owner" => {
                    filter.owner =
                        Some(Uuid::parse_str(&v).map_err(|_| "owner must be a valid UUID")?)
                }
                // "state" => filter.state = Some(imgapi::ImageState.pa
                "name" => filter.name = Some(v),
                "version" => filter.version = Some(v),
                "public" => {
                    filter.public = Some(
                        bool::from_str(&v).map_err(|_| "public must be either true or false")?,
                    )
                }
                "os" => {
                    filter.os = Some(imgapi::OperatingSystem::from_str(&v).map_err(|_| {
                        "os must be one of: smartos, linux, windows, bsd, illumos, other"
                    })?)
                }
                "type" => filter.image_type = Some(v),
                "tag" => todo!(),
                "billing_tag" => match filter.billing_tag {
                    Some(ref mut tags) => tags.push(v),
                    None => filter.billing_tag = Some(vec![v]),
                },
                "limit" => {
                    filter.limit = Some(u32::from_str(&v).map_err(|_| "limit must be an integer")?)
                }
                "marker" => todo!(),
                _ => return Err(format!("unexpected query filter: {}", arg).into()),
            }
        }
    }

    let images = imgapi::blocking::list(Some(&filter))?;
    println!("found {} image(s) matching filter", images.len());

    Ok(())
}
