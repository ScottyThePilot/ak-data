//! Options that specify where and how to interpret files as Arknights' game data.
//! Not applicable when parsing local files.
//!
//! Creating gamedata from a remote repository currently uses GitHub's API,
//! and it's compatible with any other repository hosts right now.
//!
//! If you are not using an authorized application to perform the remote requests,
//! you may run into 403 Forbidden errors due to GitHub ratelimiting you. You can instead
//! use [`GameData::from_local`][crate::game_data::GameData::from_local] to parse local game files.

#[doc(no_inline)] pub use octocrab;
#[doc(no_inline)] pub use octocrab::{Octocrab, OctocrabBuilder};

use chrono::{DateTime, Utc};
use octocrab::models::repos::RepoCommit;

use crate::format::DataFile;
use crate::game_data::GameData;

use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;



#[derive(Debug, Error, Clone, Copy)]
#[error("expected one of \"en_US\", \"ja_JP\", \"ko_KR\", \"zh_CN\", or \"zh_TW\"")]
pub struct ParseRegionError;

/// Represents which region folder to pull files from when grabbing game data from a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
  /// `en_US`
  EnUS,
  /// `ja_JP`
  JaJP,
  /// `ko_KR`
  KoKR,
  /// `zh_CN`
  ZhCN,
  /// `zh_TW`
  ZhTW
}

impl Region {
  pub fn to_str(self) -> &'static str {
    match self {
      Region::EnUS => "en_US",
      Region::JaJP => "ja_JP",
      Region::KoKR => "ko_KR",
      Region::ZhCN => "zh_CN",
      Region::ZhTW => "zh_TW"
    }
  }
}

impl Default for Region {
  #[inline]
  fn default() -> Self {
    Region::EnUS
  }
}

impl FromStr for Region {
  type Err = ParseRegionError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "en_US" => Ok(Region::EnUS),
      "ja_JP" => Ok(Region::JaJP),
      "ko_KR" => Ok(Region::KoKR),
      "zh_CN" => Ok(Region::ZhCN),
      "zh_TW" => Ok(Region::ZhTW),
      _ => Err(ParseRegionError)
    }
  }
}

impl fmt::Display for Region {
  #[inline]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.to_str())
  }
}

/// Options that specify where and how to interpret files as Arknights' game data.
#[derive(Debug, Clone)]
pub struct Options {
  /// The owner (`0`) and repository (`1`) of a GitHub repository to grab gamedata from.
  pub repository: (String, String),
  /// The branch of that repository to grab gamedata from.
  pub branch: String,
  /// The region subfolder of that repository to pull files from.
  pub region: Region,
  /// The octocrab instance used when making API requests to GitHub.
  pub instance: Octocrab
}

impl Options {
  /// Defaults to <https://github.com/Kengxxiao/ArknightsGameData>
  pub const DEFAULT_REPOSITORY: (&'static str, &'static str) = ("Kengxxiao", "ArknightsGameData");
  /// Defaults to `master`.
  pub const DEFAULT_BRANCH: &'static str = "master";
  /// Defaults to `en_US`.
  pub const DEFAULT_REGION: Region = Region::EnUS;

  pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
    Options {
      repository: (owner.into(), repo.into()),
      branch: Self::DEFAULT_BRANCH.to_owned(),
      region: Region::default(),
      instance: Octocrab::default()
    }
  }

  pub fn branch(self, branch: impl Into<String>) -> Self {
    Options {
      repository: self.repository,
      branch: branch.into(),
      region: self.region,
      instance: self.instance
    }
  }

  pub fn region(self, region: Region) -> Self {
    Options {
      repository: self.repository,
      branch: self.branch,
      region,
      instance: self.instance
    }
  }

  pub async fn get_last_updated(&self) -> Result<DateTime<Utc>, crate::Error> {
    let Options { repository: (owner, repo), branch, .. } = self;
    let repo_handle = self.instance.repos(owner, repo);
    let commits_list = repo_handle.list_commits().branch(branch).send().await?;
    commits_list.into_iter()
      .find_map(get_commit_last_updated)
      .ok_or(crate::Error::CannotFindUpdateTime)
  }

  /// Equivalent to [`GameData::from_remote`]
  pub async fn request_game_data(&self) -> Result<GameData, crate::Error> {
    let (data_files, last_updated) = tokio::try_join!(
      crate::format::DataFiles::from_remote(self),
      self.get_last_updated()
    )?;

    Ok(data_files.into_game_data(Some(last_updated)))
  }

  /// Patches the given `GameData` if the data it is based on is out of date.
  /// Replaces `self` and returns it if it was out of date.
  pub async fn patch_game_data(&self, game_data: &mut GameData) -> Result<Option<GameData>, crate::Error> {
    let last_updated = self.get_last_updated().await?;
    if game_data.is_outdated(last_updated) {
      let data_files = crate::format::DataFiles::from_remote(self).await?;
      let game_data = std::mem::replace(game_data, data_files.into_game_data(Some(last_updated)));
      Ok(Some(game_data))
    } else {
      Ok(None)
    }
  }
}

impl Default for Options {
  fn default() -> Self {
    let (owner, repo) = Self::DEFAULT_REPOSITORY;
    Options::new(owner, repo)
  }
}

fn get_commit_last_updated(repo_commit: RepoCommit) -> Option<DateTime<Utc>> {
  repo_commit.commit.author.and_then(|author| author.date)
}

pub(crate) async fn get_data_file_remote<T: DataFile>(options: &Options) -> Result<T, crate::Error> {
  let Options { repository: (owner, repo), branch, region, .. } = options;
  let repo_handle = options.instance.repos(owner, repo);
  let content_items = repo_handle.get_content().r#ref(branch)
    .path(format!("{region}/gamedata/{}", T::LOCATION))
    .send().await?;
  let content = content_items.items.into_iter().next()
    .ok_or(crate::Error::InvalidResponseContents)?;
  let blob: Blob = options.instance.get(content.links.git, None::<&()>).await?;
  let value = serde_json::from_slice(&blob.into_bytes()?)?;
  Ok(value)
}

pub(crate) async fn get_data_file_local<T: DataFile + Send + 'static>(gamedata_dir: &Path) -> Result<T, crate::Error> {
  let path = gamedata_dir.join(T::LOCATION);
  tokio::task::spawn_blocking(move || -> Result<T, crate::Error> {
    let reader = BufReader::new(File::open(path)?);
    let item = serde_json::from_reader(reader)?;
    Ok(item)
  }).await.unwrap()
}



#[derive(Debug, Serialize, Deserialize)]
struct Blob {
  sha: String,
  node_id: String,
  size: u64,
  url: String,
  content: String,
  encoding: String
}

impl Blob {
  fn into_bytes(self) -> Result<Vec<u8>, base64::DecodeError> {
    base64::decode(self.content.replace(char::is_whitespace, ""))
  }
}
