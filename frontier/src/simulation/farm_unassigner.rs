use super::*;

use commons::grid::Grid;

const HANDLE: &str = "farm_unassigner_sim";
const BATCH_SIZE: usize = 1024;

#[derive(Clone, Debug)]
struct Farmer {
    name: String,
    farm: V2<usize>,
}

pub struct FarmUnassignerSim {
    game_tx: UpdateSender<Game>,
}

impl Step for FarmUnassignerSim {
    fn name(&self) -> &'static str {
        HANDLE
    }

    fn step(&mut self, _: u128) {
        block_on(self.step_async())
    }
}

impl FarmUnassignerSim {
    pub fn new(game_tx: &UpdateSender<Game>) -> FarmUnassignerSim {
        FarmUnassignerSim {
            game_tx: game_tx.clone_with_handle(HANDLE),
        }
    }

    async fn step_async(&mut self) {
        let farmers = self.get_farmers().await;
        for batch in farmers.chunks(BATCH_SIZE) {
            self.unassign_non_existent_farms(batch.to_vec()).await
        }
    }

    async fn get_farmers(&mut self) -> Vec<Farmer> {
        self.game_tx.update(get_farmer).await
    }

    async fn unassign_non_existent_farms(&mut self, farmers: Vec<Farmer>) {
        self.game_tx
            .update(move |game| unassign_non_existent_farms(game, farmers))
            .await
    }
}

fn get_farmer(game: &mut Game) -> Vec<Farmer> {
    game.game_state()
        .avatars
        .values()
        .flat_map(as_farmer)
        .collect()
}

fn as_farmer(avatar: &Avatar) -> Option<Farmer> {
    let farm = match avatar.farm {
        Some(farm) => farm,
        None => return None,
    };
    Some(Farmer {
        name: avatar.name.clone(),
        farm,
    })
}

fn unassign_non_existent_farms(game: &mut Game, farmers: Vec<Farmer>) {
    for farmer in farmers {
        unassign_non_existent_farm(game, &farmer)
    }
}

fn unassign_non_existent_farm(game: &mut Game, farmer: &Farmer) {
    if let Some(WorldCell {
        object: WorldObject::Farm { .. },
        ..
    }) = game.game_state().world.get_cell(&farmer.farm)
    {
    } else {
        match game.mut_state().avatars.get_mut(&farmer.name) {
            Some(Avatar { farm, .. }) if *farm == Some(farmer.farm) => *farm = None,
            _ => (),
        };
    }
}
