#![allow(unused_parens)]

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

mod wasm_instant;
mod ai;
mod realtime_graph;
mod client_seen_pushes;
mod round_end_predictor;
mod draw_commands;

use std::time::Duration;
use std::cell::RefCell;

use std::collections::{VecDeque, BTreeMap};
use crossy_multi_core::map::{RowType, RowWithY};
use crossy_multi_core::player::{PushInfo, MoveState};
use draw_commands::DrawCommands;
use froggy_rand::FroggyRand;
use realtime_graph::RealtimeGraph;
use round_end_predictor::RoundEndPredictor;
use wasm_instant::{WasmInstant, WasmDateInstant};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use client_seen_pushes::*;

use crossy_multi_core::*;
use crossy_multi_core::game::PlayerId;
use crossy_multi_core::crossy_ruleset::{AliveState, RulesState};

use crate::draw_commands::{DrawCommand, DrawCoords, DrawColour, DrawType};

struct ConsoleDebugLogger();
impl crossy_multi_core::DebugLogger for ConsoleDebugLogger {
    fn log(&self, logline: &str) {
        log!("{}", logline);
    }
}

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const TICK_INTERVAL_US : u32 = 16_666;

#[derive(Debug)]
pub struct LocalPlayerInfo {
    player_id: game::PlayerId,
    buffered_input : Input,
}

//const TIME_REQUEST_INTERVAL : u32 = 13;
const TIME_REQUEST_INTERVAL : u32 = 2;

const RUN_TELEMETRY : bool = true;
const RUN_PING_LATENCY_UPDATES : bool = true;

#[wasm_bindgen]
pub struct Client {
    client_start : WasmInstant,
    server_start: Option<WasmInstant>,
    server_start_date : WasmDateInstant,
    estimated_latency_us : f32,

    server_start_lerping: WasmInstant,
    estimated_latency_us_lerping : f32,

    timeline: timeline::Timeline,

    local_player_info : Option<LocalPlayerInfo>,
    last_sent_frame_id : u32,

    // This seems like a super hacky solution
    untrusted_rules_state : Option<crossy_ruleset::RulesState>,
    lkg_rules_state : Option<crossy_ruleset::RulesState>,

    queued_time_info : Option<interop::TimeRequestEnd>,

    queued_server_linden_messages : VecDeque<interop::LindenServerTick>,

    ai_agent : Option<RefCell<Box<dyn ai::AIAgent>>>,

    telemetry_buffer : TelemetryBuffer,
    
    server_time_offset_graph : RealtimeGraph,
    server_message_count_graph : RealtimeGraph,

    client_seen_pushes : ClientSeenPushManager,
    round_end_predictor : RoundEndPredictor,

    tick_id : u32,
}

#[wasm_bindgen]
impl Client {

    #[wasm_bindgen(constructor)]
    pub fn new(seed : &str, server_frame_id : i32, _server_time_us : i32, estimated_latency_us : i32) -> Self {
        // Setup statics
        console_error_panic_hook::set_once();
        crossy_multi_core::set_debug_logger(Box::new(ConsoleDebugLogger()));

        let estimated_frame_delta = estimated_latency_us / 16_666;
        let estimated_server_current_frame_id = (server_frame_id as i32 + estimated_frame_delta) as u32;
        let estimated_server_time_us = estimated_server_current_frame_id * TICK_INTERVAL_US;
        //let timeline = timeline::Timeline::from_server_parts(seed, server_frame_id as u32, server_frame_id as u32 * TICK_INTERVAL_US, vec![], Default::default());
        let timeline = timeline::Timeline::from_server_parts(seed, 0, 0, Default::default(), RulesState::new(Default::default()));

        // Estimate server start
        let client_start = WasmInstant::now();
        //let server_start = client_start - Duration::from_micros((server_time_us + estimated_latency_us) as u64);
        let server_start = client_start - Duration::from_micros((estimated_server_time_us) as u64);

        let client_start_date = WasmDateInstant::now();
        //let server_start_date = client_start_date - Duration::from_micros((server_time_us + estimated_latency_us) as u64);
        let server_start_date = client_start_date - Duration::from_micros((estimated_server_time_us) as u64);

        log!("Constructing client : estimated latency {}, server frame_id {}, estimated now server_frame_id {}", estimated_latency_us, server_frame_id, estimated_server_current_frame_id);

        let mut telemetry_buffer = TelemetryBuffer::new(RUN_TELEMETRY);
        telemetry_buffer.push(interop::TelemetryMessage::LatencyEstimate(interop::Telemetry_LatencyEstimate {
            estimated_latency_us,
            estimated_frame_delta,
            estimated_server_current_frame_id,
        }));

        Client {
            timeline,
            client_start,

            // @TODO REMEMBER MEEEE
            server_start : None,

            server_start_date,
            server_start_lerping : server_start,
            estimated_latency_us : estimated_latency_us as f32,
            estimated_latency_us_lerping : estimated_latency_us as f32,
            local_player_info : None,
            last_sent_frame_id : server_frame_id as u32,
            queued_time_info: Default::default(),
            queued_server_linden_messages: Default::default(),
            ai_agent : None,
            telemetry_buffer,

            server_time_offset_graph : RealtimeGraph::new(60 * 10),
            server_message_count_graph : RealtimeGraph::new(60 * 10),

            client_seen_pushes : ClientSeenPushManager::default(),
            round_end_predictor : RoundEndPredictor::default(),

            tick_id : 0,

            untrusted_rules_state: None,
            lkg_rules_state: None,
        } 
    }

    pub fn join(&mut self, player_id : u32) {
        self.local_player_info = Some(LocalPlayerInfo {
            player_id : PlayerId(player_id as u8),
            buffered_input : Input::None,
        })
    }

    pub fn buffer_input_json(&mut self, input_json : &str) {
        if (input_json == "\"Kill\"") {
            panic!("Manual kill signal");
        }

        let input = serde_json::from_str(input_json).map_err(|e| log!("{} {:?}", input_json, e)).unwrap_or(Input::None);
        self.buffer_input(input);
    }

    fn buffer_input(&mut self, input : Input) {
        self.local_player_info.as_mut().map(|x| {
            //if input != Input::None {
            if input != Input::None && x.buffered_input == Input::None {
                x.buffered_input = input;
            }
        });
    }

    pub fn get_top_frame_id(&self) -> u32 {
        self.timeline.top_state().frame_id
    }

    pub fn tick(&mut self) {
        self.tick_id += 1;

        if let Some(server_start) = self.server_start {
            //debug_log!("Ticking!");
            loop {
                let current_time = server_start.elapsed();
                let current_time_us = current_time.as_micros() as u32;

                let last_time = self.timeline.top_state().time_us;
                //let delta_time = current_time_us.saturating_sub(last_time);
                //if (delta_time > TICK_INTERVAL_US)
                if (current_time_us > last_time)
                {
                    self.tick_inner();
                }
                else
                {
                    break;
                }
            }

            if let Some(top_linden_message) = self.queued_server_linden_messages.front() {
                /*
                if (top_linden_message.latest.frame_id > self.timeline.top_state().frame_id)
                {
                    panic!("Got message with latest frame id in the future!!\n\n frame_id {}\n top states {:?}\n\n state {:?}", top_linden_message.latest.frame_id, self.timeline.top_state(), top_linden_message.latest);
                }
                */

                let delta = self.timeline.top_state().frame_id as f32 - top_linden_message.latest.frame_id as f32;
                self.server_time_offset_graph.push(delta);
            }
            else
            {
                self.server_time_offset_graph.repeat();
            }

            self.server_message_count_graph.push(self.queued_server_linden_messages.len() as f32);

            let mut requeued_server_messages = VecDeque::new();

            while let Some(linden_server_tick) = self.queued_server_linden_messages.pop_back() {
                //log!("{:#?}", linden_server_tick);
                let delta_input_server_frame_times = linden_server_tick.delta_inputs.iter().map(|x| x.frame_id).collect::<Vec<_>>();

                self.telemetry_buffer.push(interop::TelemetryMessage::ClientReceiveEvent(interop::Telemetry_ClientReceiveEvent {
                    server_send_frame_id : linden_server_tick.latest.frame_id,
                    receive_frame_id : self.timeline.top_state().frame_id,
                    //delta_input_server_frame_times,
                    delta_input_server_frame_times_count: delta_input_server_frame_times.len() as u32,
                    delta_input_server_frame_times_min: delta_input_server_frame_times.first().cloned(),
                    delta_input_server_frame_times_max: delta_input_server_frame_times.last().cloned(),
                }));

                if (!self.try_process_linden_server_message(&linden_server_tick)) {
                    requeued_server_messages.push_front(linden_server_tick);
                }
            }

            self.queued_server_linden_messages = requeued_server_messages;
        }

        self.process_time_info();

        self.client_seen_pushes.tick(&self.timeline);
    }

    pub fn tick_inner(&mut self) {
        let mut player_inputs = PlayerInputs::new();

        let mut can_move = false;
        let local_player_id = self.local_player_info.as_ref().map(|x| x.player_id);
        local_player_id.map(|id| {
            self.timeline.top_state().get_player(id).map(|player| {
                can_move = player.can_move();
            });
        });

        if (self.local_player_info.is_some())
        {
            let mut local_input = Input::None;

            if (can_move)
            {
                if let Some(ai_refcell) = &self.ai_agent {
                    let mut ai = ai_refcell.borrow_mut();
                    local_input = ai.think(&self.timeline.top_state(), &self.timeline.map);
                }
                else 
                {
                    let local_player_info = self.local_player_info.as_mut().unwrap();
                    if (local_player_info.buffered_input != Input::None)
                    {
                        local_input = local_player_info.buffered_input;
                        local_player_info.buffered_input = Input::None;
                    }
                }
            }

            player_inputs.set(self.local_player_info.as_ref().unwrap().player_id, local_input);
        }

        self.timeline.tick(Some(player_inputs), TICK_INTERVAL_US)
    }

    fn process_time_info(&mut self)
    {
        if let Some(time_request_end) = self.queued_time_info.take() {
            if (!RUN_PING_LATENCY_UPDATES) {
                return;
            }

            //debug_log!("Processing time info!");

            let t0 = time_request_end.client_send_time_us as i64;
            let t1 = time_request_end.server_receive_time_us as i64;
            let t2 = time_request_end.server_send_time_us as i64;
            let t3 = time_request_end.client_receive_time_us as i64;

            let time_now_us = WasmInstant::now().saturating_duration_since(self.client_start).as_micros() as u32;
            let holding_time_us = time_now_us - t3 as u32;

            let total_time_in_flight = t3 - t0;
            let total_time_on_server = t2 - t1;
            let ed = (total_time_in_flight - total_time_on_server) / 2;

            let latency_lerp_k = 50. / TIME_REQUEST_INTERVAL as f32;

            self.estimated_latency_us_lerping = dan_lerp(self.estimated_latency_us_lerping, ed as f32, latency_lerp_k);
            //let estimated_latency_us_blablah = estimated_latency_us_lerping;
            let estimated_latency_us_blablah = ed;

            let estimated_server_time_us = t2 as u32 + estimated_latency_us_blablah as u32 + holding_time_us;

            //log!("Holding time {}us", holding_time);

            //let new_server_start = self.client_start + Duration::from_micros(t3 as u64 + holding_time as u64) - Duration::from_micros(estimated_server_time_us as u64);
            //let new_server_start = self.client_start + Duration::from_micros(t3 as u64) - Duration::from_micros(estimated_server_time_us as u64);

            // Server time now = t2 + estimated latency + holding_time
            // Client time now = client_start + time_now
            let new_server_start = self.client_start + Duration::from_micros(time_now_us as u64) - Duration::from_micros(estimated_server_time_us as u64);

            // @TODO DAN TEMP
            if (self.server_start.is_none())
            {
                debug_log!("Setting latency up: t2 : {}, estimated_latency_ms: {}, holding_time_ms: {}", t2, estimated_latency_us_blablah / 1000, holding_time_us / 1000);
                self.server_start = Some(new_server_start);
            }

            //debug_log!("FRAME WE SHOULD BE ON FROM LATEST PING {}", new_server_start.elapsed().as_micros() as u32 / TICK_INTERVAL_US);
            //debug_log!("FRAME WE ARE ACTUALLY ON {}", self.get_top_frame_id());

            let server_start_lerp_k_up = 500. / TIME_REQUEST_INTERVAL as f32;
            let server_start_lerp_k_down = 500. / TIME_REQUEST_INTERVAL as f32;
            self.server_start_lerping = WasmInstant(dan_lerp_directional(self.server_start_lerping.0 as f32, new_server_start.0 as f32, server_start_lerp_k_up, server_start_lerp_k_down) as i128);

            //log!("estimated latency {}ms", self.estimated_latency_us as f32 / 1000.);
            //log!("estimated server start {}delta_ms", self.server_start.0 as f32 / 1000.);

            let current_time = self.server_start.unwrap().elapsed();
            let current_client_time_us = current_time.as_micros() as u32;

            let current_date_time = self.server_start_date.elapsed();
            let current_client_date_time_us = current_date_time.as_micros() as u32;

            self.telemetry_buffer.push(interop::TelemetryMessage::PingOutcome(interop::Telemetry_PingOutcome {
                unlerped_estimated_latency_us : ed,
                unlerped_estimated_frame_delta : ed / 16_666,
                estimated_latency_us : self.estimated_latency_us_lerping,
                estimated_frame_delta :self.estimated_latency_us_lerping / 16_666.0,

                estimated_server_time_us : estimated_server_time_us,
                estimated_server_current_frame_id : estimated_server_time_us / 16_666,

                current_client_time_ms : current_client_time_us / 1000,
                current_client_date_time_ms : current_client_date_time_us / 1000,
            }));
        }
    }

    fn try_process_linden_server_message(&mut self, linden_server_tick : &interop::LindenServerTick) -> bool
    {
        //let mut should_reset = self.trusted_rules_state.as_ref().map(|x| !x.same_variant(&linden_server_tick.rules_state)).unwrap_or(false);
        //should_reset |= self.timeline.top_state().player_states.count_populated() != linden_server_tick.latest.states.len();

        let should_reset = false;

        if (should_reset)
        {
            log!("Resetting!");
            self.timeline = timeline::Timeline::from_server_parts_exact_seed(
                self.timeline.map.get_seed(),
                linden_server_tick.latest.frame_id,
                linden_server_tick.latest.time_us,
                linden_server_tick.latest.states.clone(),
                linden_server_tick.rules_state.clone());
        }
        else
        {
            if let Some(client_state_at_lkg_time) = (self.timeline.try_get_state(linden_server_tick.lkg_state.frame_id))
            {
                let mismatch_player_states = linden_server_tick.lkg_state.player_states != client_state_at_lkg_time.player_states;
                let mismatch_rulestate = linden_server_tick.lkg_state.rules_state != client_state_at_lkg_time.rules_state;
                if (mismatch_player_states || mismatch_rulestate)
                {
                    log!("Tick Id: {}", self.tick_id);

                    if (mismatch_player_states)
                    {
                        log!("Mismatch in LKG! frame_id {}", client_state_at_lkg_time.frame_id);
                        log!("Local at lkg time {:#?}", client_state_at_lkg_time.player_states);
                        log!("LKG {:#?}", linden_server_tick.lkg_state.player_states);
                        log!("Rebasing... {:?}", linden_server_tick.lkg_state);
                    }
                    else
                    {
                        log!("Mismatch in rules\n\nlocal:\n{:#?} \n\n lkg:\n {:#?}", client_state_at_lkg_time.rules_state, linden_server_tick.lkg_state.rules_state);
                        //log!("Mismatch rules");
                    }


                    // TODO We do a ton of extra work, we recalculate from lkg with current inputs then run propate inputs from server.
                    self.timeline = self.timeline.rebase(&linden_server_tick.lkg_state);
                    //log!("Local {:#?}", client_state_at_lkg_time.player_states);
                    //log!("Remote {:#?}", linden_server_tick.lkg_state);
                    //self.timeline.states
                }
            }
            //log!("Propagating inputs {:#?}", linden_server_tick.delta_inputs);

            if !self.timeline.try_propagate_inputs(linden_server_tick.delta_inputs.clone(), false) {
                return false;
            }
        }

        self.untrusted_rules_state = Some(linden_server_tick.rules_state.clone());
        self.lkg_rules_state = Some(linden_server_tick.lkg_state.rules_state.clone());

        true
    }

    fn get_round_id(&self) -> u8 {
        self.untrusted_rules_state.as_ref().map(|x| x.fst.get_round_id()).unwrap_or(0)
    }


    pub fn estimate_time_from_frame_id(&self) -> f32 {
        //let time_ms = self.get_top_frame_id() as f32 / 16.66;
        //time_ms / 1000.0
        self.get_top_frame_id() as f32 / 60.0
    }

    pub fn recv(&mut self, server_tick : &[u8])
    {
        if let Some(deserialized) = try_deserialize_message(server_tick)
        {
            self.recv_internal(deserialized);
        }
    }

    fn recv_internal(&mut self, message : interop::CrossyMessage)
    {
        let client_receive_time_us = WasmInstant::now().saturating_duration_since(self.client_start).as_micros() as u32;
        match message {
            interop::CrossyMessage::TimeResponsePacket(time_info) => {
                self.queued_time_info = Some(interop::TimeRequestEnd {
                    client_receive_time_us,
                    client_send_time_us : time_info.client_send_time_us,
                    server_receive_time_us : time_info.server_receive_time_us,
                    server_send_time_us : time_info.server_send_time_us,
                });

                //log!("Got time response, {:#?}", self.queued_time_info);
            },

            interop::CrossyMessage::LindenServerTick(linden_server_tick) => {
                self.queued_server_linden_messages.push_front(linden_server_tick);
            }
            _ => {},
        }
    }

    pub fn get_client_message(&mut self) -> Vec<u8>
    {
        let message = self.get_client_message_internal();
        flexbuffers::to_vec(message).unwrap()
    }

    fn get_client_message_internal(&mut self) -> interop::CrossyMessage
    {
        let mut ticks = Vec::new();

        while self.last_sent_frame_id <= self.timeline.top_state().frame_id {
            if let Some(timeline_state) = self.timeline.try_get_state(self.last_sent_frame_id)
            {
                let input = self.local_player_info
                    .as_ref()
                    .map(|x| timeline_state.player_inputs.get(x.player_id))
                    .unwrap_or(Input::None);

                ticks.push(interop::ClientTick {
                    time_us: timeline_state.time_us,
                    frame_id: timeline_state.frame_id,
                    input: input,
                });
            }

            self.last_sent_frame_id += 1;
        }

        interop::CrossyMessage::ClientTick(ticks)
    }

    pub fn get_server_time_offset_graph_json(&self) -> String
    {
        let snapshot = self.server_time_offset_graph.snapshot();
        serde_json::to_string(&snapshot).unwrap()
    }

    pub fn get_server_message_count_graph_json(&self) -> String
    {
        let snapshot = self.server_message_count_graph.snapshot();
        serde_json::to_string(&snapshot).unwrap()
    }


    pub fn should_get_time_request(&self) -> bool {
        let frame_id = self.timeline.top_state().frame_id;
        frame_id % TIME_REQUEST_INTERVAL == 0
    }

    pub fn get_time_request(&self) -> Vec<u8>
    {
        let message = self.get_time_request_internal();
        flexbuffers::to_vec(message).unwrap()
    }

    fn get_time_request_internal(&self) -> interop::CrossyMessage
    {
        let client_send_time_us = WasmInstant::now().saturating_duration_since(self.client_start).as_micros() as u32;
        interop::CrossyMessage::TimeRequestPacket(interop::TimeRequestPacket {
            client_send_time_us,
        })
    }

    fn get_telemetry_message_internal(&mut self) -> interop::CrossyMessage
    {
        let mut events = Vec::new();
        std::mem::swap(&mut events, &mut self.telemetry_buffer.buffer);

        interop::CrossyMessage::TelemetryMessagePackage(interop::TelemetryMessagePackage{
            messages: events,
        })
    }

    pub fn get_telemetry_message(&mut self) -> Vec<u8>
    {
        let message = self.get_telemetry_message_internal();
        flexbuffers::to_vec(message).unwrap()
    }

    pub fn has_telemetry_messages(&self) -> bool {
        !self.telemetry_buffer.buffer.is_empty()
    }

    pub fn get_players_json(&self) -> String
    {
        let time_us = self.timeline.top_state().time_us;
        let players : Vec<_> = self.timeline.top_state().get_valid_player_states()
            .iter()
            .map(|x| x.to_public(self.get_round_id(), time_us, &self.timeline.map))
            .collect();

        serde_json::to_string(&players).unwrap()
    }

    // Return -1 if no local player
    pub fn get_local_player_id(&self) -> i32 {
        self.local_player_info.as_ref().map(|x| x.player_id.0 as i32).unwrap_or(-1)
    }

    pub fn get_rules_state_json(&self) -> String {
        match self.get_latest_server_rules_state() {
            Some(x) => {
                serde_json::to_string(x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }

    fn get_latest_server_rules_state(&self) -> Option<&crossy_ruleset::RulesState> {
        self.untrusted_rules_state.as_ref()
    }

    pub fn get_rows_json(&mut self) -> String {
        serde_json::to_string(&self.get_rows()).unwrap()
    }

    fn get_rows(&mut self) -> Vec<RowWithY> {
        let screen_y = self.untrusted_rules_state.as_ref().map(|x| x.fst.get_screen_y()).unwrap_or(0);
        let round_id = self.get_round_id();
        self.timeline.map.get_row_view(round_id, screen_y)
    }

    pub fn get_cars_json(&self) -> String {
        let cars = self.timeline.map.get_cars(self.get_round_id(), self.timeline.top_state().time_us);
        serde_json::to_string(&cars).unwrap()
    }

    pub fn get_lillipads_json(&self) -> String {
        let lillipads = self.timeline.map.get_lillipads(self.get_round_id(), self.timeline.top_state().time_us);
        serde_json::to_string(&lillipads).unwrap()
    }

    pub fn get_bushes_row_json(&self, row_y : i32) -> String {
        let round_id = self.get_round_id();
        let row = self.timeline.map.get_row(round_id, row_y);
        if let RowType::Bushes(bush_descr) = row.row_type {
            let hydrated = bush_descr.hydrate();
            serde_json::to_string(&hydrated).unwrap()
        }
        else {
            panic!("Tried to hydrate bushes over a non-bush row! round_id {} | row_y {} \n rows {:#?}", round_id, row_y, self.timeline.map);
        }
    }

    pub fn get_wall_width(&self, row_y : i32) -> i32 {
        let round_id = self.get_round_id();
        let row = self.timeline.map.get_row(round_id, row_y);
        row.wall_width().map(|x| x as i32).unwrap_or(-1)
    }

    pub fn player_alive_state_json(&self, player_id : u32) -> String {
        serde_json::to_string(&self.player_alive_state(player_id)).unwrap()
    }

    fn player_alive_state(&self, player_id : u32) -> AliveState
    {
        // We have to be careful here.
        // We dont want to tell the client a player is dead if they could possibly "come back alive".
        // For remote players we want to wait for confirmation from the server.
        // For local player we can probably make this decision earlier. (Weird edge case where player pushing you in gets interrupted before they can?)

        self.get_latest_server_rules_state().map(|x| {
            x.fst.get_player_alive(PlayerId(player_id as u8))
        }).unwrap_or(AliveState::NotInGame)
    }

    pub fn is_river(&self, y : f64) -> bool {
        match self.timeline.map.get_row(self.get_round_id(), y.round() as i32).row_type
        {
            map::RowType::River(_) => true,
            _ => false,
        }
    }

    pub fn is_bush(&self, y : f64) -> bool {
        match self.timeline.map.get_row(self.get_round_id(), y.round() as i32).row_type
        {
            map::RowType::Bushes(_) => true,
            _ => false,
        }
    }

    pub fn is_path(&self, y : f64) -> bool {
        match self.timeline.map.get_row(self.get_round_id(), y.round() as i32).row_type
        {
            map::RowType::Path{..} => true,
            map::RowType::Stands => true,
            map::RowType::StartingBarrier => true,
            _ => false,
        }
    }

    pub fn set_ai(&mut self, ai_config : &str) {
        if (self.local_player_info.is_none())
        {
            log!("No local player to set ai on");
            return;
        }

        let local_player_id = self.local_player_info.as_ref().unwrap().player_id;

        let lower = ai_config.to_lowercase();
        match lower.as_str() {
            "none" => {
                log!("Setting ai agent to none");
                self.ai_agent = None;
            },
            "go_up" => {
                log!("Setting ai agent to 'go_up'");
                self.ai_agent = Some(RefCell::new(Box::new(ai::go_up::GoUpAI::new(local_player_id))));
            },
            "back_and_forth" => {
                log!("Setting ai agent to 'back_and_forth'");
                self.ai_agent = Some(RefCell::new(Box::new(ai::BackAndForth::new(local_player_id))));
            },
            _ => {
                log!("Unknown ai agent {}", ai_config);
            }
        }
    }

    fn get_draw_commands(&self) -> Option<DrawCommands> {
        let mut draw_state = DrawCommands::default();
        if let Some(x) = self.local_player_info.as_ref() {
            if (self.player_alive_state(x.player_id.0 as u32) != AliveState::Alive) {
                return None;
            }

            if let Some(commands) = self.ai_agent.as_ref().map(|x| x.borrow().get_drawstate().clone()) {
                for command in commands.commands {
                    draw_state.commands.push(command);
                }
            }
        }

        for (x, y) in &self.client_seen_pushes.pushes {
            draw_state.commands.push(DrawCommand {
                pos: DrawCoords::from_precise(y.pusher_pos),
                draw_type: DrawType::Line(DrawCoords::from_precise(y.pushee_pos)),
                colour: match &y.state {
                    PushDataState::Valid => DrawColour::Green,
                    PushDataState::Invalid => DrawColour::Red,
                    PushDataState::Archived => DrawColour::Grey,
                }
            });
        }

        Some(draw_state)
    }

    pub fn get_draw_commands_json(&self) -> String {
        match self.get_draw_commands() {
            Some(x) => {
                serde_json::to_string(&x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }

    fn get_lilly_drawstate(&self) -> Option<Vec<LillyOverlay>> {
        self.local_player_info.as_ref().and_then(|x| {
            if (self.player_alive_state(x.player_id.0 as u32) != AliveState::Alive) {
                None
            }
            else {
                let top_state = self.timeline.top_state();
                top_state.get_player(x.player_id).and_then(|player| {
                    match &player.move_state {
                        player::MoveState::Stationary => {
                            let (precise_coords, on_lillypad) = match &player.pos {
                                Pos::Coord(coord_pos) => {
                                    (coord_pos.to_precise(), false)
                                },
                                Pos::Lillipad(lilly_id) => {
                                    let x = self.timeline.map.get_lillipad_screen_x(top_state.time_us, &lilly_id);
                                    (PreciseCoords {
                                        x,
                                        y : lilly_id.y,
                                    }, true)
                                },
                                _ => {
                                    unreachable!()
                                }
                            };

                            let lilly_moves = get_lilly_moves(&precise_coords, on_lillypad, top_state.get_round_id(), top_state.time_us, &self.timeline.map, &top_state.rules_state);
                            Some(lilly_moves)

                        }
                        _ => {
                            None
                        }
                    }
                })
            }
        })
    }

    pub fn get_lilly_drawstate_json(&self) -> String {
        match self.get_lilly_drawstate() {
            Some(x) => {
                serde_json::to_string(&x).unwrap()
            }
            _ => {
                "".to_owned()
            }
        }
    }


    pub fn rand_for_prop_unit(&self, x : i32, y : i32, scenario : &str) -> f32 {
        let rand = FroggyRand::from_hash((self.timeline.map.get_seed(), self.get_lkg_round_identifier()));
        rand.gen_unit((x, y, scenario)) as f32
    }
}

impl Client {
    pub fn get_lkg_round_identifier(&self) -> RoundIdentifier {
        RoundIdentifier::from_rulesstate(self.lkg_rules_state.as_ref().unwrap())
    }

    pub fn get_untrusted_round_identifier(&self) -> RoundIdentifier {
        RoundIdentifier::from_rulesstate(self.untrusted_rules_state.as_ref().unwrap())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct RoundIdentifier
{
    pub game_id : u32,
    pub round_id : u8,
}

impl RoundIdentifier {
    fn from_rulesstate(rules_state : &RulesState) -> Self {
        Self {
            game_id : rules_state.game_id,
            round_id : rules_state.fst.get_round_id(),
        }
    }
}

#[derive(serde::Serialize, Debug, Clone)]
struct LillyOverlay {
    precise_coords : PreciseCoords,
    input : Input,
}

fn get_lilly_moves(initial_pos : &PreciseCoords, on_lilly: bool, round_id : u8, time_us : u32, map : &map::Map, rule_state: &RulesState) -> Vec<LillyOverlay>
{
    let mut moves = vec![];

    for input in &ALL_INPUTS {
        let mut applied = initial_pos.apply_input(*input);
        if let Some(lilly) = map.lillipad_at_pos(round_id, time_us, applied, rule_state) {
            let screen_x = map.get_lillipad_screen_x(time_us, &lilly);
            moves.push(LillyOverlay {
                precise_coords: PreciseCoords {
                    x : screen_x,
                    y : applied.y,
                },
                input: *input,
            });
        }
        else {
            let row_is_river = match &map.get_row(round_id, applied.y).row_type {
                RowType::River(_) => true,
                _ => false,
            };

            if on_lilly && !row_is_river {
                applied.x = applied.x.round();
                moves.push(LillyOverlay {
                    precise_coords: applied,
                    input: *input,
                });
            }
        }
    }

    moves
}

fn try_deserialize_message(buffer : &[u8]) -> Option<interop::CrossyMessage>
{
    let reader = flexbuffers::Reader::get_root(buffer).map_err(|e| log!("{:?}", e)).ok()?;
    interop::CrossyMessage::deserialize(reader).map_err(|e| log!("{:?}", e)).ok()
}

fn dan_lerp(x0 : f32, x : f32, k : f32) -> f32 {
    (x0 * (k-1.0) + x) / k
}

fn dan_lerp_directional(x0 : f32, x : f32, k_up : f32, k_down : f32) -> f32 {
    let k = if (x > x0) {
        k_up
    }
    else {
        k_down
    };

    dan_lerp(x0, x, k)
}

fn dan_lerp_snap_thresh(x0 : f32, x : f32, k : f32, snap_thresh : f32) -> f32 {
    if (x0 - x).abs() > snap_thresh {
        x
    }
    else
    {
        dan_lerp(x0, x, k)
    }
}

#[derive(Debug)]
struct TelemetryBuffer
{
    enabled : bool,
    buffer: Vec<crossy_multi_core::interop::TelemetryMessage>
}

impl TelemetryBuffer
{
    fn new(enabled : bool) -> Self {
        Self { enabled, buffer: Default::default() }
    }

    fn push(&mut self, message: crossy_multi_core::interop::TelemetryMessage) {
        if (self.enabled) {
            self.buffer.push(message);
        }
    }
}
