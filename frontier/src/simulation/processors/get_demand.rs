use super::*;
use crate::settlement::Settlement;

pub struct GetDemand<F>
where
    F: Fn(&Settlement) -> Vec<Demand>,
{
    demand_fn: F,
}

#[async_trait]
impl<F> Processor for GetDemand<F>
where
    F: Fn(&Settlement) -> Vec<Demand> + Send,
{
    async fn process(&mut self, mut state: State, instruction: &Instruction) -> State {
        let settlement = match instruction {
            Instruction::GetDemand(settlement) => settlement,
            _ => return state,
        };
        for demand in (self.demand_fn)(settlement) {
            state.instructions.push(Instruction::GetRoutes(demand))
        }
        state
    }
}

impl<F> GetDemand<F>
where
    F: Fn(&Settlement) -> Vec<Demand>,
{
    pub fn new(demand_fn: F) -> GetDemand<F> {
        GetDemand { demand_fn }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::resource::Resource;

    #[test]
    fn should_add_instruction_for_each_demand() {
        fn demand() -> Vec<Demand> {
            vec![
                Demand {
                    resource: Resource::Crops,
                    quantity: 1,
                    sources: 1,
                    ..Demand::default()
                },
                Demand {
                    resource: Resource::Truffles,
                    quantity: 1,
                    sources: 1,
                    ..Demand::default()
                },
            ]
        }

        let demand_fn = |_: &Settlement| demand();

        let mut processor = GetDemand::new(demand_fn);
        let state = block_on(processor.process(
            State::default(),
            &Instruction::GetDemand(Settlement::default()),
        ));

        let expected = demand();
        assert_eq!(
            state.instructions,
            vec![
                Instruction::GetRoutes(expected[0]),
                Instruction::GetRoutes(expected[1]),
            ]
        );
    }

    #[test]
    fn should_add_instruction_for_demand_with_zero_quantity() {
        let demand = Demand {
            resource: Resource::Crops,
            quantity: 0,
            sources: 1,
            ..Demand::default()
        };
        let demand_fn = |_: &Settlement| vec![demand];

        let mut processor = GetDemand::new(demand_fn);
        let state = block_on(processor.process(
            State::default(),
            &Instruction::GetDemand(Settlement::default()),
        ));

        assert_eq!(state.instructions, vec![Instruction::GetRoutes(demand)]);
    }

    #[test]
    fn should_add_instruction_for_demand_with_zero_sources() {
        let demand = Demand {
            resource: Resource::Crops,
            quantity: 1,
            sources: 0,
            ..Demand::default()
        };
        let demand_fn = |_: &Settlement| vec![demand];

        let mut processor = GetDemand::new(demand_fn);
        let state = block_on(processor.process(
            State::default(),
            &Instruction::GetDemand(Settlement::default()),
        ));

        assert_eq!(state.instructions, vec![Instruction::GetRoutes(demand)]);
    }
}
