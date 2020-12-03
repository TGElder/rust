use crate::nation::NationDescription;
use crate::traits::send::SendNations;
use commons::async_trait::async_trait;

#[async_trait]
pub trait GetNationDescription {
    async fn get_nation_description(&self, position: String) -> Option<NationDescription>;
}

#[async_trait]
impl<T> GetNationDescription for T
where
    T: SendNations + Sync,
{
    async fn get_nation_description(&self, position: String) -> Option<NationDescription> {
        self.send_nations(move |nations| {
            nations
                .get(&position)
                .map(|nation| nation.description())
                .cloned()
        })
        .await
    }
}