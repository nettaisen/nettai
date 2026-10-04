--!strict
-- BN6's stock rules (docs/design/rules-in-luau.md §2.2): the systems a BN6
-- player plays by, in the order the framework calls them. The framework
-- (Rust) still runs the BN6 rules no system has taken over yet; the systems
-- take them slice by slice (§8).

local beast = require("./beast/system")
local cross = require("./cross/system")
local dark_chips = require("./dark-chips/system")
local folder = require("./folder/system")
local emotion = require("./emotion/system")
local forms = require("./forms/system")
local navicust = require("./navicust/system")
local patch_cards = require("./patch-cards/system")

return define.ruleset {
    id = "stock",
    stock = true,
    systems = {
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
        forms.system,
        -- Beast Out's turns.
        beast,
        -- Its emotions: tired at the start, Full Synchro on a counter, the
        -- NaviCust's swing bug.
        emotion.system,
        -- The folder's rules, which tools ask (the simulation never does).
        folder,
        -- The dark chips: their bug frag, substitute and HP bug.
        dark_chips,
    },
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    banners = require("./banners"),
    berserk = require("./berserk"),
    buster = require("./buster"),
    chip_use = require("./chip-use"),
    cross_special = require("./cross-special"),
    custom_screen = require("./custom-screen"),
    elements = require("./elements"),
    flow = require("./flow"),
    lockon = require("./lockon").rules,
    math = require("./math"),
    navicust = require("./navicust/section"),
    panels = require("./panels"),
    pools = require("./pools"),
    reactions = require("./reactions"),
    roles = require("./roles"),
    sp_chips = require("./sp-chips"),
    status = require("./status").rules,
}
