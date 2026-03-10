// @cpt-req:cpt-cf-resource-group-dod-cdc-consumer:p2
// @cpt-req:cpt-cf-resource-group-dod-full-resync:p2

pub mod event;
pub mod handler;
pub mod resync;

#[cfg(test)]
mod handler_test;
