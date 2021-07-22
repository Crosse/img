use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::{form_urlencoded, Url};

pub use uuid::Uuid;

pub use chrono::DateTime;
use chrono::Utc;

pub mod blocking;

pub const JOYENT_IMGAPI_URL: &str = "https://images.joyent.com/images";

#[derive(Debug, Default, Clone)]
pub struct ImageFilter {
    /// Only allow access to images visible to this account.
    ///
    /// A user can see: (a) their own images, (b) activated public images, and (c) activated private
    /// images for which they are on the ACL. Note that "activated" is different than "active" (see
    /// [`ImageState`]). This field is only relevant for 'mode=dc' IMGAPI servers.
    pub account: Option<Uuid>,

    /// The image channel to use. If not provided the server-side default channel is used.
    ///
    /// Use '*' to list in all channels.
    pub channel: Option<String>,

    /// Whether to include administrative fields (e.g. files.*.stor) in the returned image objects.
    ///
    /// For IMGAPI servers using 'mode' other than dc, auth is required to use this.
    pub include_admin_fields: Option<bool>,

    /// Only list images owned by this account.
    pub owner: Option<Uuid>,

    /// List images with the given state.
    pub state: Option<ImageState>,

    /// List images with the given name.
    ///
    /// Prefix with `~` to do a substring match (case-sensitive). E.g., `~foo`.
    pub name: Option<String>,

    /// List images with the given version.
    ///
    /// Prefix with `~` to do a substring match (case-sensitive). E.g., `~foo`.
    pub version: Option<String>,

    /// List just public or just private images. The default is to list all images.
    pub public: Option<bool>,

    /// List images with the given [`OperatingSystem`].
    pub os: Option<OperatingSystem>,

    /// List images of the given type. The value can be prefixed with `!` to exclude that type.
    pub image_type: Option<String>,

    /// List images by tags.
    ///
    /// The key for each item should not include the initial `tag.` shown in the [IMGAPI
    /// documentation](https://images.joyent.com/docs/#ListImages). For example, if an image is
    /// tagged as `cloud=private`, then the filter to be added would look like this:
    ///
    /// ```
    /// use std::collections::HashMap;
    /// let mut tags: HashMap<String, String> = HashMap::new();
    /// tags.insert("cloud".to_string(), "private".to_string());
    /// ```
    ///
    /// More than one tag can be specified for the same search. Multiple tags are interpreted as a
    /// logical AND, meaning that each of the images returned is tagged with each of the values
    /// provided.
    pub tag: Option<HashMap<String, String>>,

    pub billing_tag: Option<Vec<String>>,

    /// Maximum number of images to return.
    ///
    /// Images are sorted by creation date (ASC) by default. The default (and maximum) limit value
    /// is 1000.
    pub limit: Option<u32>,
    // XXX: handle markers for pagination
    // pub marker: Option<???>,
}

impl std::string::ToString for ImageFilter {
    fn to_string(&self) -> String {
        macro_rules! add_param {
            ($param:ident, $collection:ident) => {
                add_param!($param, stringify!($param), $collection);
            };
            ($param:ident, $query_name:expr, $collection:ident) => {
                add_param!($param, stringify!($param), to_string, $collection);
            };
            ($param:ident, $query_name:expr, $val_func:ident, $collection:ident) => {
                if let Some(v) = &self.$param {
                    $collection.append_pair($query_name, &v.$val_func());
                }
            };
        }

        let mut qp = form_urlencoded::Serializer::new(String::new());

        add_param!(account, qp);
        add_param!(channel, qp);
        add_param!(include_admin_fields, "inclAdminFields", qp);
        add_param!(owner, qp);
        add_param!(state, qp);
        add_param!(name, qp);
        add_param!(version, qp);
        add_param!(public, qp);
        add_param!(os, "os", as_param, qp);
        add_param!(image_type, qp);
        add_param!(limit, qp);

        if let Some(val) = &self.tag {
            for (k, v) in val.iter() {
                qp.append_pair(&format!("tag.{}", k), v);
            }
        }

        if let Some(val) = &self.billing_tag {
            for v in val.iter() {
                qp.append_pair("billing_tag", v);
            }
        }

        qp.finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// Version of the manifest format/spec. The current value is 2.
    pub v: u32,

    /// The unique identifier for a UUID. This is set by the IMGAPI server.
    pub uuid: Uuid,

    /// The UUID of the owner of this image (the account that created it).
    pub owner: Uuid,

    /// A short name for this image.
    ///
    /// Note: Max 512 characters (though practical usage should be much shorter). No uniqueness
    /// guarantee.
    pub name: String,

    /// A version string for this image.
    ///
    /// Note: Max 128 characters. No uniqueness guarantee.
    pub version: String,

    /// A short description of the image.
    pub description: Option<String>,

    /// Homepage URL where users can find more information about the image.
    pub homepage: Option<Url>,

    /// URL of the End User License Agreement (EULA) for the image.
    pub eula: Option<Url>,

    /// Indicates if the image has an icon file. If not present, then no icon is present.
    pub icon: Option<bool>,

    /// The current state of the image. One of 'active', 'unactivated', 'disabled', 'creating',
    /// 'failed'.
    pub state: ImageState,

    /// An object with details on image creation failure.
    ///
    /// This only set when state is [`State::Failed`].
    pub error: Option<ImageError>,

    /// Indicates if this image is available for provisioning.
    pub disabled: bool,

    /// Indicates if this image is publicly available.
    pub public: bool,

    /// The date at which the image is activated.
    pub published_at: Option<DateTime<Utc>>,

    #[serde(rename = "type")]
    /// The image type.
    pub image_type: String,

    /// The OS family this image provides.
    pub os: String,

    /// The origin image UUID if this is an incremental image.
    pub origin: Option<Uuid>,

    /// An array with a single object describing the image file.
    pub files: Vec<File>,

    /// An array of account UUIDs given access to a private image. The field is only relevant to
    /// private images.
    pub acl: Option<Vec<Uuid>>,

    /// A list of users for which passwords should be generated for provisioning.
    pub users: Option<Vec<User>>,

    /// A list of tags that can be used by operators for additional billing processing.
    pub billing_tags: Option<Vec<String>>,

    /// An object that defines a collection of properties that is used by other APIs to evaluate
    /// where should customer VMs be placed.
    pub traits: Option<Value>,

    /// An object of key/value pairs that allows clients to categorize images by any given criteria.
    pub tags: Option<HashMap<String, Value>>,

    /// Indicates whether to generate passwords for the users in the [`users`] field.  If `None`,
    /// the field should be assumed to mean `true`.
    pub generate_passwords: Option<bool>,

    /// A list of inherited directories (other than the defaults for the brand).
    pub inherited_directories: Option<Vec<String>>,

    /// NIC driver used by this VM image. Only required for [`ImageType::Zvol`] images.
    pub nic_driver: Option<String>,

    /// Disk driver used by this VM image. Only required for [`ImageType::Zvol`] images.
    pub disk_driver: Option<String>,

    /// The QEMU CPU model used by this VM image. Only required for [`ImageType::Zvol`] images.
    pub cpu_type: Option<String>,

    /// The size (in MiB) of this VM image's disk. Only required for [`ImageType::Zvol`] images.
    pub image_size: Option<u32>,

    /// Array of channel names to which this image belongs.
    pub channels: Option<Vec<String>>,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// The current state of the image.
pub enum ImageState {
    /// The image is ready for use, i.e. VMs can be provisioned using this image.
    Active,

    /// The image has not yet been activated.
    Unactivated,

    /// The image is disabled.
    ///
    /// This will be the state if the image is activated, but also disabled == true.
    Disabled,

    /// A state for a placeholder image while an image is being asynchronously created.
    Creating,

    /// A state for a placeholder image indicating that asynchronous image creation failed.
    Failed,
}

impl fmt::Display for ImageState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Active => "active",
            Self::Unactivated => "unactivated",
            Self::Disabled => "disabled",
            Self::Creating => "creating",
            Self::Failed => "failed",
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// An object providing details on failure of some asynchronous image action.
pub struct ImageError {
    /// String description of the error.
    pub message: String,

    /// A "CamelCase" string error code.
    pub code: Option<String>,

    /// A stack trace giving context for the error.
    ///
    /// This is generally considered internal implementation detail, only there to assist with
    /// debugging and error classification.
    pub stack: Option<String>,
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ImageErrorCode {
    /// This typically means that the target KVM VM (e.g. Linux) has old guest tools that pre-date
    /// the image creation feature.
    ///
    /// Guest tools can be upgraded with installers at
    /// https://download.joyent.com/pub/guest-tools/. Other possibilities are: a boot time greater
    /// than the 5 minute timeout or a bug or crash in the image preparation script.
    PrepareImageDidNotRun,

    /// Origin image data could not be found for the VM.
    ///
    /// Either the link to the image from which the VM was created has been broken (e.g. via 'zfs
    /// promote' or migration) or there is some problem in either the 'image_uuid' value from vmadm
    /// get or in imgadm's DB of manifest info for that image.
    VmHasNoOrigin,

    /// Indicates an error due to functionality that isn't currently supported.
    ///
    /// An example is that custom image creation of a VM based on a custom image isn't currently
    /// supported.
    NotSupported,
}

impl fmt::Display for ImageErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PrepareImageDidNotRun => "PrepareImageDidNotRun",
            Self::VmHasNoOrigin => "VmHasNoOrigin",
            Self::NotSupported => "NotSupported",
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// An image file that makes up part or all of an image.
pub struct File {
    /// SHA-1 hex digest of the file content. Used for upload/download corruption checking.
    pub sha1: String,

    /// Number of bytes. Maximum 20GiB.
    pub size: u64,

    /// The type of file compression used by the file.
    pub compression: Compression,

    /// The ZFS internal unique identifier for this dataset's snapshot.
    ///
    /// This identifier is available via `zfs get guid SNAPSHOT`, e.g. `zfs get guid
    /// zones/f669428c-a939-11e2-a485-b790efc0f0c1@final`.
    pub dataset_guid: Option<Uuid>,

    #[serde(skip)]
    pub stor: Option<String>,

    /// Docker digest of the file contents. Only used when [`Image::image_type`] is 'docker'.
    pub digest: Option<String>,

    #[serde(rename = "uncompressedDigest")]
    /// Docker digest of the uncompressed file contents. Only used when [`Image::image_type`] is 'docker'.
    pub uncompressed_digest: Option<String>,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// The type of compression used to compress image files.
pub enum Compression {
    Bzip2,
    Gzip,
    None,
}

impl fmt::Display for Compression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Bzip2 => "bzip2",
            Self::Gzip => "gzip",
            Self::None => "none",
        }
        .fmt(f)
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum ImageType {
    #[serde(rename = "zone-dataset")]
    /// A ZFS dataset used to create a new SmartOS (illumos?) zone.
    ZoneDataset,

    #[serde(rename = "lx-dataset")]
    /// An lx-brand image.
    LxDataset,

    #[serde(rename = "zvol")]
    /// A virtual machine image for use by KVM or Bhyve.
    Zvol,

    #[serde(rename = "other")]
    /// An image that serves any other specific purpose.
    Other,
}

impl fmt::Display for ImageType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ZoneDataset => "SmartOS zone dataset",
            Self::LxDataset => "Lx-brand dataset",
            Self::Zvol => "zvol",
            Self::Other => "Other",
        }
        .fmt(f)
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperatingSystem {
    SmartOS,
    Windows,
    Linux,
    BSD,
    Illumos,
    Other,
}

impl OperatingSystem {
    fn as_param(&self) -> &str {
        match self {
            Self::SmartOS => "smartos",
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::BSD => "bsd",
            Self::Illumos => "illumos",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParseOsError {}

impl fmt::Display for ParseOsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid operating system")
    }
}

impl FromStr for OperatingSystem {
    type Err = ParseOsError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "smartos" => Ok(Self::SmartOS),
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "bsd" => Ok(Self::BSD),
            "illumos" => Ok(Self::Illumos),
            "other" => Ok(Self::Other),
            _ => Err(ParseOsError {}),
        }
    }
}

impl fmt::Display for OperatingSystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::SmartOS => "SmartOS",
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::BSD => "BSD",
            Self::Illumos => "illumos",
            Self::Other => "Other",
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirements {
    /// An array describing the minimum number of network interfaces.
    pub networks: Vec<Network>,

    /// Defines the SmartOS "brand" that is required to provision with this image.
    pub brand: Option<String>,

    /// Indicates that provisioning with this image requires that an SSH public key be provided.
    pub ssh_key: Option<bool>,

    /// The minimum RAM (in MiB) required to provision the image.
    pub min_ram: Option<u32>,

    /// The maximum RAM (in MiB) the image may be provisioned with.
    pub max_ram: Option<u32>,

    /// The minimum required SmartOS platform on which this image can be used.
    ///
    /// It is a mapping of major "SDC Version" to the SmartOS platform timestamp.
    pub min_platform: Option<HashMap<String, String>>,

    /// The maximum required SmartOS platform on which this image can be used.
    ///
    /// It is a mapping of major "SDC Version" to the SmartOS platform timestamp.
    pub max_platform: Option<HashMap<String, String>>,

    /// The boot ROM image to use.
    pub boot_rom: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub name: String,
    pub description: String,
}

/// The boot ROM an image uses.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootRom {
    Bios,
    Uefi,
}

impl fmt::Display for BootRom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Bios => "BIOS",
            Self::Uefi => "UEFI",
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub name: String,
}
