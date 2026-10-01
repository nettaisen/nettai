//! The content pack's data as scripts read it: the frozen global `data`
//! (typed for editors in content/bn6/core.d.luau). Entities appear as
//! their pack files write them (field names, enums as names, sprites as
//! `"CC-II"`), keyed by their ids:
//!
//! ```text
//! data.chips[id]            a chip's record and its own data (gun_del_sol, ...)
//! data.objects.attachments[id], .rocks[id], .absorbed_sprites[kind],
//!             .body_overlays[id], .sun_beam_looks[look], .projectiles[kind],
//!             .flying_shots[kind], .shock_waves[variant], .name_looks[name_id],
//!             .boomerangs[id], .sword_waves[kind]
//! data.objects.kinds[name]  an object kind a script implements: pool, index, script
//! data.rules.buster_recovery[rapid * 6 + open]   the buster's recovery (byte_80209CC)
//! data.rules.field_regions[region - 0x80]        a whole-field region's panel condition
//! data.rules.cross_palettes[form]   MegaMan's palette in each Cross (byte_80203EA)
//! data.rules.sine[step]     the sine table: 256 steps a turn, 1.0 = 0x100 (math_sinTable)
//! data.regions[region]      a hit region's panels, [dx, dy] each (PanelOffsetListsPointerTable)
//! ```

use bn6_content_api::{Data, DataKey};
use serde::Serialize;

use crate::content::Content;

/// A serializable value as data (its pack-file form).
fn value<T: Serialize>(v: &T) -> Data {
    from_json(serde_json::to_value(v).expect("content data serializes"))
}

fn from_json(v: serde_json::Value) -> Data {
    use serde_json::Value as J;
    match v {
        J::Null => Data::Nil,
        J::Bool(b) => Data::Bool(b),
        J::Number(n) => Data::Int(n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)).expect("content numbers are integers")),
        J::String(s) => Data::Str(s),
        J::Array(items) => Data::List(items.into_iter().map(from_json).collect()),
        J::Object(fields) => Data::Map(fields.into_iter().map(|(k, v)| (DataKey::Str(k), from_json(v))).collect()),
    }
}

/// A table by id.
fn by_id<T>(items: impl IntoIterator<Item = (i64, T)>, f: impl Fn(&T) -> Data) -> Data {
    Data::Map(items.into_iter().map(|(id, v)| (DataKey::Int(id), f(&v))).collect())
}

/// The `data` global for `content`.
pub fn script_data(c: &Content) -> Data {
    let o = &c.objects;
    let objects = Data::map([
        ("attachments", by_id(o.attachments.iter().map(|a| (a.id as i64, a)), |a| value(*a))),
        ("rocks", by_id(o.rocks.iter().map(|r| (r.id as i64, r)), |r| value(*r))),
        ("absorbed_sprites", by_id(o.absorbed_sprites.iter().enumerate().map(|(i, s)| (i as i64, s)), |s| value(*s))),
        ("body_overlays", by_id(o.body_overlays.iter().map(|b| (b.id as i64, b)), |b| value(*b))),
        ("sun_beam_looks", by_id(o.sun_beam_looks.iter().enumerate().map(|(i, s)| (i as i64, s)), |s| value(*s))),
        ("boomerangs", by_id(o.boomerangs.iter().map(|b| (b.id as i64, b)), |b| value(*b))),
        ("projectiles", by_id(o.projectiles.iter().map(|p| (p.id as i64, p)), |p| value(*p))),
        ("flying_shots", by_id(o.flying_shots.iter().map(|p| (p.id as i64, p)), |p| value(*p))),
        ("sword_waves", by_id(o.sword_waves.iter().map(|w| (w.id as i64, w)), |w| value(*w))),
        ("name_looks", by_id(o.name_looks.iter().map(|l| (l.name_id as i64, l)), |l| value(*l))),
        ("shock_waves", by_id(o.shock_waves.iter().map(|w| (w.id as i64, w)), |w| value(*w))),
        (
            "kinds",
            Data::Map(
                o.kinds
                    .iter()
                    .map(|k| {
                        let d = Data::map([
                            ("pool", Data::Str(k.pool.name().into())),
                            ("index", Data::Int(k.index as i64)),
                            ("script", Data::Str(k.script.clone())),
                        ]);
                        (DataKey::Str(k.name.clone()), d)
                    })
                    .collect(),
            ),
        ),
    ]);
    let recovery = c.rules.buster_recovery.iter().flatten().enumerate().map(|(i, &t)| (i as i64, t));
    let field_regions = c.rules.field_regions.iter().enumerate().map(|(i, r)| (i as i64, r));
    let palettes = c.rules.cross_palettes.iter().enumerate().map(|(i, &v)| (i as i64, v));
    let rules = Data::map([
        ("buster_recovery", by_id(recovery, |&t| Data::Int(t as i64))),
        ("field_regions", by_id(field_regions, |r| value(*r))),
        ("cross_palettes", by_id(palettes, |&v| Data::Int(v as i64))),
        ("sine", by_id(c.rules.sine.iter().enumerate().map(|(i, &v)| (i as i64, v)), |&v| Data::Int(v as i64))),
    ]);
    Data::map([
        ("chips", by_id(c.chips.iter().filter_map(|x| Some((x.id? as i64, x))), |x| value(*x))),
        ("regions", by_id(c.regions.iter().enumerate().map(|(i, r)| (i as i64, r)), |r| value(*r))),
        ("objects", objects),
        ("rules", rules),
    ])
}
