//!82D30CC0 lifecycle ordering. Toolkit refresh is a real prerequisite service.
pub trait ExitServices {
    ///Run complete original82D81610 against the actual toolkit/contact owner.
    ///Its active-contact branch must not be replaced by a reset/no-op.
    fn refresh_toolkit_contacts(&mut self);
    ///After refresh, zero actual toolkit320 and30452/30456/30460/30464/30468.
    fn clear_toolkit_exit_fields(&mut self);
    fn ground_geometry_active(&self) -> bool;
    ///Actual Groundstate76 geometry service virtual12 release.
    fn release_ground_geometry(&mut self);
    fn clear_ground_geometry_active(&mut self);
}
pub fn exit(services: &mut impl ExitServices) {
    services.refresh_toolkit_contacts();
    services.clear_toolkit_exit_fields();
    if services.ground_geometry_active() {
        services.release_ground_geometry();
        services.clear_ground_geometry_active();
    }
}
