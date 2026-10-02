//! Stages: where a round is fought (`define.stage`, docs/design/
//! content-model-v2.md §3.7). A stage holds its own panel layout and what
//! it places on the field when the round starts; nothing about it has a
//! number.

use nettai_content_api::{KindHandle, RecordHandle};

use crate::field::PanelType;
use crate::sound::SoundId;

/// A panel layout: panel types `[y - 1][x - 1]` over the playable 6x3.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PanelLayout {
    pub rows: [[PanelType; 6]; 3],
}

/// A stage (the original's 16-byte battle settings record, with the panel
/// layout and the actor list it names).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct StageData {
    pub layout: PanelLayout,
    /// Panel column pattern (which columns belong to which side).
    pub panel_pattern: u8,
    /// Its music, outside link battles (none: the round starts no music).
    pub music: Option<SoundId>,
    /// The background a round on it shows unless its settings say
    /// otherwise.
    pub background: super::BackgroundId,
    /// Battle mode (0 = netbattle).
    pub mode: u8,
    pub battle_number: u8,
    /// `effects` bits (`setup::effects`) a round on it runs with unless its
    /// settings say otherwise.
    pub effects: u32,
    /// Who and what it places when the round starts, in order
    /// (`sub_8007368`).
    pub actors: Vec<ActorEntry>,
}

/// Something a stage places on the field when the round starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActorEntry {
    /// What to place.
    pub place: Place,
    /// The side, for navis. Field objects take the side of their panel.
    pub side: u8,
    /// Panel.
    pub x: u8,
    pub y: u8,
    /// Which of the kind's variants (a record of the kind's: a rock's).
    pub variant: Option<RecordHandle>,
    /// The entry's raw argument, for a kind whose spawner only leaves it
    /// in a register (the Guardian statue's).
    pub argument: u8,
}

/// What a stage's entry places.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// A player navi (`sub_80073CC`): the side's.
    Navi,
    /// A field object, by its kind's `place`.
    Kind(KindHandle),
}

/// The panel type a layout names (its serde name: `normal`, `road_up`).
fn panel_type(name: &str) -> Option<PanelType> {
    PanelType::ALL.into_iter().find(|t| serde_json::to_value(t).ok().and_then(|v| v.as_str().map(|s| s == name)).unwrap_or(false))
}

/// A stage definition as the engine holds it. `kind` and `record` give the
/// handles of the kinds and records its entries name.
pub(crate) fn read(
    d: &nettai_content_api::Definition,
    assets: &nettai_content_api::AssetNames,
    kind: impl Fn(&str) -> Option<KindHandle>,
    record: impl Fn(&str) -> Option<RecordHandle>,
) -> Result<StageData, nettai_content_api::ContentError> {
    use nettai_content_api::{AssetKind, ContentError, Data, Registry};
    let what = |m: String| ContentError::new(format!("{}.luau: stage {}: {m}", d.module, d.key));
    let spec = &d.spec;
    let byte = |v: &Data, field: &str| -> Result<u8, ContentError> {
        match v {
            Data::Int(i) => u8::try_from(*i).map_err(|_| what(format!("`{field}` {i} is not a byte"))),
            _ => Err(what(format!("needs `{field}` (a number)"))),
        }
    };

    let Data::List(rows) = spec.field("layout") else {
        return Err(what("`layout` is three rows of six panel types".into()));
    };
    if rows.len() != 3 {
        return Err(what(format!("`layout` has {} rows, not 3", rows.len())));
    }
    let mut layout = PanelLayout::default();
    for (y, row) in rows.iter().enumerate() {
        let names: Vec<&str> = row.str().map(|r| r.split_whitespace().collect()).unwrap_or_default();
        if names.len() != 6 {
            return Err(what(format!("layout row {} has {} panels, not 6", y + 1, names.len())));
        }
        for (x, name) in names.iter().enumerate() {
            layout.rows[y][x] =
                panel_type(name).ok_or_else(|| what(format!("{name:?} is not a panel type")))?;
        }
    }

    let music = match spec.field("music") {
        Data::Nil => None,
        Data::Asset(AssetKind::Sound, name) => Some(SoundId(
            assets.handle(AssetKind::Sound, name).ok_or_else(|| what(format!("the packs have no sound {name:?}")))?,
        )),
        _ => return Err(what("`music` is a sound (asset.sound(...)), or nothing".into())),
    };
    let background = match spec.field("background") {
        Data::Asset(AssetKind::Background, name) => super::BackgroundId(
            assets.handle(AssetKind::Background, name).ok_or_else(|| what(format!("the packs have no background {name:?}")))?,
        ),
        _ => return Err(what("needs a `background` (asset.background(...))".into())),
    };
    let effects = match spec.field("effects") {
        Data::Nil => 0,
        Data::Int(i) => u32::try_from(*i).map_err(|_| what(format!("`effects` {i:#x} is not a flags word")))?,
        _ => return Err(what("`effects` is a flags word".into())),
    };

    let entries = match spec.field("actors") {
        Data::List(entries) => entries.as_slice(),
        Data::Map(m) if m.is_empty() => &[],
        _ => return Err(what("`actors` is a list of what the stage places".into())),
    };
    let mut actors = Vec::with_capacity(entries.len());
    for (i, e) in entries.iter().enumerate() {
        let at = |m: &str| what(format!("actors[{}]: {m}", i + 1));
        let place = match e.field("place") {
            Data::Str(s) if s == "navi" => Place::Navi,
            Data::Ref(Registry::Kind, key) => Place::Kind(kind(key).ok_or_else(|| at(&format!("the kind {key:?} is not in the content")))?),
            _ => return Err(at("`place` is \"navi\" or a kind with a `place`")),
        };
        let variant = match e.field("variant") {
            Data::Nil => None,
            Data::Ref(Registry::Record, key) => Some(record(key).ok_or_else(|| at(&format!("the record {key:?} is not in the content")))?),
            _ => return Err(at("`variant` is one of its kind's variants (a record)")),
        };
        if place == Place::Navi && variant.is_some() {
            return Err(at("a navi has no `variant`"));
        }
        let opt = |field: &str| -> Result<u8, ContentError> {
            match e.field(field) {
                Data::Nil => Ok(0),
                v => byte(v, field),
            }
        };
        let (x, y) = (byte(e.field("x"), "x")?, byte(e.field("y"), "y")?);
        actors.push(ActorEntry { place, side: opt("side")?, x, y, variant, argument: opt("argument")? });
    }

    Ok(StageData {
        layout,
        panel_pattern: byte(spec.field("panel_pattern"), "panel_pattern")?,
        music,
        background,
        mode: byte(spec.field("mode"), "mode")?,
        battle_number: byte(spec.field("battle_number"), "battle_number")?,
        effects,
        actors,
    })
}
