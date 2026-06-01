use crate::model::{Target, TargetKind};

pub fn is_init_target(target: &Target) -> bool {
    target.kind == TargetKind::Init
}
