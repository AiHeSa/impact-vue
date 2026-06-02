pub mod cross_module;
pub mod graph_builder;
pub mod import_resolver;
pub mod init_phase;
pub mod path_utils;
pub mod pruning;
pub mod step;
pub mod target_resolver;

use crate::model::{Direction, ImpactGraph, SourceFileIr, Target};

pub fn resolve_watch(watch: &str) -> Option<Target> {
    target_resolver::resolve_watch(watch)
}

pub fn build_graph(all_irs: &[SourceFileIr], target: &Target, direction: &Direction) -> ImpactGraph {
    graph_builder::build_full_graph(all_irs, target, direction)
}

pub fn is_init_target(target: &Target) -> bool {
    init_phase::is_init_target(target)
}

pub fn prune_graph(graph: &mut ImpactGraph, target: &Target) {
    pruning::prune_main_chain(graph, target);
}
