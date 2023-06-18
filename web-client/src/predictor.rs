use crossy_multi_core::{player_id_map::PlayerIdMap, GameState, player::PlayerState, timeline::Timeline, crossy_ruleset::{RulesState, GameConfig}, PlayerId};


pub struct Predictor
{
    config : GameConfig,
    player_predictors : PlayerIdMap<PlayerPredictor>,
}

impl Predictor {
    pub fn new(config : GameConfig) -> Self {
        Self {
            config,
            player_predictors: Default::default(),
        }
    }

    pub fn tick(&mut self, timeline: &Timeline, latest_server_time_u32 : u32, latest_server_rules : &RulesState, lkg_state : &GameState) {
        let top = timeline.top_state();

        // TODO replace with get_by_Frame_id
        let latest_server_state = timeline.get_state_before_eq_us(latest_server_time_u32);

        // Ensure we have the union of the players in the three inputs in our map
        {
            for (id, s) in top.player_states.iter() {
                _ = self.get_player_predictor(id);
            }

            if let Some(state) = latest_server_state {
                for (id, s) in state.player_states.iter() {
                    _ = self.get_player_predictor(id);
                }
            }

            for (id, s) in lkg_state.player_states.iter() {
                _ = self.get_player_predictor(id);
            }
        }

        // TODO write an iter_mut()
        let ids = self.player_predictors.valid_ids();
        for (id) in ids {
            self.player_predictors.get_mut(id).unwrap().tick(latest_state, latest_dead, lkg_state, lkg_dead)
        }
    }

    fn get_player_predictor(&mut self, id : PlayerId) -> &mut PlayerPredictor {
        if (!self.player_predictors.contains(id)) {
            self.player_predictors.set(id, PlayerPredictor::default());
        }

            self.player_predictors.get_mut(id).unwrap()
    }

    pub fn round_is_over(&self)
    {
    }

    pub fn round_might_be_cooling_down(&self) -> bool {
        let mut alive_predicted_count = 0;
        for (_, player_predictor) in self.player_predictors.iter() {
            if (!player_predictor.predict_is_dead()) {
                alive_predicted_count += 1;
            }
        }

        alive_predicted_count < self.config.minimum_players
    }

    pub fn round_def_is_cooling_down(&self) -> bool {
        let mut alive_count = 0;
        for (_, player_predictor) in self.player_predictors.iter() {
            if (!player_predictor.actually_dead()) {
                alive_count += 1;
            }
        }

        alive_count < self.config.minimum_players
    }
}

#[derive(Default)]
pub struct PlayerPredictor
{
}

impl PlayerPredictor {

    // actually dead implies predict dead
    pub fn predict_is_dead(&self) -> bool {
    }

    pub fn actually_dead(&self) -> bool {

    }

    pub fn tick(&mut self, latest_state : PlayerState, latest_dead : bool, lkg_state : PlayerState, lkg_dead : bool) {
    }
}