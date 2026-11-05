//! Normal gameplay camera rendering endpoint. Simulation owns the cadence of
//! CameraRuntime::advance after its complete physical/animation publication.
pub(crate) mod angle;
mod settings;
mod collision;
mod world_query;
mod shot_data;
mod stock_names;
mod shake_data;
mod trajectory;
mod subject;
mod graph_subject;
mod graph_conditions;
mod graph;
mod runtime;
mod publication;
pub(crate) use publication::{
    snapshot as publish_camera_subject, CameraPublicationInputs, CameraStateOutput,
    CameraAnimationOutput, CameraAirOutput, CameraOffboardOutput, CameraGrindOutput,
    CameraEventsOutput, CameraPreferences,
};
pub(crate) use graph_subject::CameraGraphEnvironment;
pub(crate) use runtime::CameraRuntime;
