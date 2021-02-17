use crate::nation::NationDescription;
use crate::traits::WithNations;
use commons::async_trait::async_trait;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[async_trait]
pub trait GetNationDescription {
    async fn get_nation_description(&self, name: &str) -> Option<NationDescription>;
}

#[async_trait]
impl<T> GetNationDescription for T
where
    T: WithNations + Sync,
{
    async fn get_nation_description(&self, name: &str) -> Option<NationDescription> {
        self.with_nations(|nations| {
            nations
                .get(name)
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
    T: WithNations + Sync,
{
    async fn nation_descriptions(&self) -> Vec<NationDescription> {
        self.with_nations(|nations| {
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
    async fn random_town_name(&self, nation: &str) -> Result<String, NationNotFound>;
}

#[async_trait]
impl<T> RandomTownName for T
where
    T: WithNations + Sync,
{
    async fn random_town_name(&self, nation: &str) -> Result<String, NationNotFound> {
        self.mut_nations(|nations| {
            nations
                .get_mut(nation)
                .map(|nation| nation.get_town_name())
                .ok_or(NationNotFound {
                    nation: nation.to_string(),
                })
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
