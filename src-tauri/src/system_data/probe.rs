//! Runs one catalogue probe.

use super::catalog::Probe;
use super::expand::{expand, Lookup};
use super::measure::{measure_each, Acc};
use super::{windows, Measured};

pub struct Ctx<'a> {
    pub lookup: &'a Lookup,
    pub elevated: bool,
}

/// Measures one item. `report` is called once, or twice for the component
/// store (a quick walk first, DISM's real size later).
pub fn run(probe: &Probe, ctx: &Ctx, report: &mut dyn FnMut(Measured)) {
    match probe {
        Probe::Paths(templates) => {
            let mut acc = Acc::default();
            for template in templates.iter() {
                if let Some(path) = expand(template, ctx.lookup) {
                    acc.add_path(&path);
                }
            }
            report(acc.finish());
        }
        Probe::Each { base, subs } => report(measure_each(base, subs, ctx.lookup)),
        Probe::Find { bases, ext, depth } => {
            let mut acc = Acc::default();
            for base in bases.iter() {
                if let Some(path) = expand(base, ctx.lookup) {
                    acc.find(&path, ext, *depth);
                }
            }
            report(acc.finish());
        }
        Probe::RecycleBin => report(windows::recycle_bin()),
        Probe::ShadowStorage => report(windows::shadow_storage(ctx.elevated)),
        Probe::ComponentStore => windows::component_store(ctx, report),
    }
}
