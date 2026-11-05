//! Native input publication. Controller/graph producers remain separate stages.
pub mod angle;
pub mod animation_packet;
pub mod anticipation_intentions;
pub mod body_flip_signal;
pub mod controller;
pub mod gameplay_map;
pub mod gesture;
pub mod graph_intents;
pub mod grind_intentions;
pub mod history;
pub mod manual_intentions;
pub mod pad;
pub mod player_input_data;
pub mod power_sliding;
pub mod riding_intentions;
pub mod set_turning;
pub mod steering_intentions;
pub mod tick;
pub mod trick_intentions;
pub mod turn_conditioner;
pub mod turn_remap;
pub mod xbox;

pub mod offboard_intentions;
pub mod wipeout_intentions;

pub mod gameplay_gestures;

pub mod object_intentions;
