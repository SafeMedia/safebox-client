use crate::{Deserialize, Error, PathBuf, Serialize, XorName};

pub(crate) const USER_SESSION_KEY: &str = "user";

#[derive(Serialize)]
pub(crate) struct SimpleAccountUser {
    pub(crate) username: String,
    pub(crate) address: String,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutonomiFileMetadata {
    pub(crate) folder_path: Option<String>,
    pub(crate) file_name: Option<String>,
    pub(crate) extension: Option<String>,
    pub(crate) xorname: Option<XorName>,
    pub(crate) size: Option<u32>,
}

impl AutonomiFileMetadata {
    pub fn full_path(&self) -> Result<PathBuf, Error> {
        let mut path = PathBuf::from(self.folder_path.clone().ok_or(Error::Common(
            "Need folder_path to construct full path.".into(),
        ))?);

        path.push(format!(
            "{}.{}",
            self.file_name.clone().ok_or(Error::Common(
                "Need file_name to construct full path.".into()
            ))?,
            self.extension.clone().ok_or(Error::Common(
                "Need extension to construct full path.".into()
            ))?
        ));

        Ok(path)
    }
}
