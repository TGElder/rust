use crate::nation::NationDescription;
use crate::traits::send::SendNations;
use commons::async_trait::async_trait;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[async_trait]
pub trait GetNationDescription {
    async fn get_nation_description(&self, name: String) -> Option<NationDescription>;
}

#[async_trait]
impl<T> GetNationDescription for T
where
    T: SendNations + Sync,
{
    async fn get_nation_description(&self, name: String) -> Option<NationDescription> {
        self.send_nations(move |nations| {
            nations
                .get(&name)
                .map(|nation| nation.description())
                .cloned()
        })
        .await
    }
}

#[async_trait]
pub trait NationDescriptions {
    async fn nation_descriptions(&self) -> Vec<NationDescription>;
}

#[async_trait]
impl<T> NationDescriptions for T
where
    T: SendNations + Sync,
{
    async fn nation_descriptions(&self) -> Vec<NationDescription> {
        self.send_nations(|nations| {
            nations
                .values()
                .map(|nation| nation.description())
                .cloned()
                .collect()
        })
        .await
    }
}

#[async_trait]
pub trait RandomTownName {
    async fn random_town_name(&self, nation: String) -> Result<String, NationNotFound>;
}

#[async_trait]
impl<T> RandomTownName for T
where
    T: SendNations + Sync,
{
    async fn random_town_name(&self, nation: String) -> Result<String, NationNotFound> {
        self.send_nations(|nations| {
            nations
                .get_mut(&nation)
                .map(|nation| nation.get_town_name())
                .ok_or_else(|| NationNotFound { nation })
        })
        .await
    }
}

#[derive(Debug)]
pub struct NationNotFound {
    nation: String,
}

impl Display for NationNotFound {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown nation: {}", self.nation)
    }
}

impl Error for NationNotFound {}
