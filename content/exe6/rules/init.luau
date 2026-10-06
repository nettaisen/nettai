--!strict
-- EXE6's rules, its one definition of them (docs/design/rules-in-luau.md
-- §2.2): the parts an EXE6 player plays by (plain modules), whose hooks run in
-- this order (@exelib/rules/compose), its rule sections and its roles. The
-- framework (Rust) still runs the EXE6 rules no part has taken over yet; the
-- parts take them slice by slice (§8).

local compose = require("@exelib/rules/compose")
local beast = require("@self/beast")
local cross = require("@self/cross")
local dark_chips = require("@self/dark_chips")
local folder = require("@self/folder")
local emotion = require("@self/emotion")
local forms = require("@self/forms")
local navicust = require("@self/navicust")
local patch_cards = require("@self/patch_cards")
local save = require("@self/save")

return define.rules(compose.rules({
    -- What the save brings to the navi's stats (the base HP, the Regular
    -- memory, the sun), and a link navi's stats at its level: first, as
    -- the reload builds the stats the NaviCust's compile reads.
    save,
    -- The Crosses on the custom screen: ChargeCross's chips and the
    -- hand size.
    cross,
    -- The NaviCust: the programs a player placed, compiled into the
    -- navi's stats (docs/design/navicust.md); before the patch cards,
    -- which apply to what it made.
    navicust,
    -- The Japanese games' patch cards (docs/engine/patch-cards.md).
    patch_cards,
    -- The form changes: the Crosses', Beast Out's and Beast Over's.
    forms.part,
    -- Beast Out's turns.
    beast,
    -- Its emotions: tired at the start, Full Synchro on a counter, the
    -- NaviCust's swing bug.
    emotion.part,
    -- The folder's rules, which tools ask (the simulation never does).
    folder,
    -- The dark chips: their bug frag, substitute and HP bug.
    dark_chips,
}, {
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    banners = require("@self/banners"),
    berserk = require("@self/berserk"),
    buster = require("@self/buster"),
    chip_use = require("@self/chip_use"),
    custom_screen = require("@self/custom_screen"),
    effects = require("@self/effects"),
    elements = require("@self/elements"),
    flow = require("@self/flow"),
    fresh_stats = require("@self/fresh_stats"),
    link_pick = require("@self/link_pick"),
    lockon = require("@self/lockon").rules,
    math = require("@self/math"),
    navicust = require("@self/navicust/section"),
    panels = require("@self/panels"),
    pools = require("@self/pools"),
    reactions = require("@self/reactions"),
    roles = require("@self/roles"),
    sp_chips = require("@self/sp_chips"),
    status = require("@self/status").rules,
}))
