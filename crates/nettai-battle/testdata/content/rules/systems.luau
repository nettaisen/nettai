--!strict
-- The test content's rules (docs/design/rules-in-luau.md): two made-up
-- systems and two rulesets, for the framework's tests of per-side state,
-- player setups and hooks. Neither changes how a battle plays. A third
-- ruleset plays by BN6's patch-cards system, for its tests.

local patch_cards = require("./patch-cards")

export type CounterState = { starts: number, side: number, bonus: number }

-- Counts the round starts it sees, and keeps its side and twice its
-- player's setup bonus.
local counter = define.system {
    id = "test/counter",
    state = { starts = "u8", side = "u8", bonus = "u16" },
    setup = { bonus = "u8" },
    -- Made-up fields of the game's chips, for the extension's tests.
    extends = {
        chip = {
            test_weight = "u8",
            test_tag = { level = "u8", kind = { "a", "b" } },
        },
    },
    hooks = {
        round_start = function(side: number)
            local s = system.state() :: CounterState
            s.starts += 1
            s.side = side
            s.bonus = (system.setup() :: { bonus: number }).bonus * 2
        end,
    },
}

export type MarkerState = { mark: number }

-- Marks its side's state, to tell the rulesets apart.
local marker = define.system {
    id = "test/marker",
    state = { mark = "u8" },
    hooks = {
        round_start = function(side: number)
            local s = system.state() :: MarkerState
            s.mark = 0x40 + side
        end,
    },
}

export type WatcherState = { intakes: number, x: number, y: number }

-- Counts its side's navi intakes and keeps the navi's panel; refuses the
-- test bomb, giving the test seed instead.
local test_chips = require("../chips/test")
local watcher = define.system {
    id = "test/watcher",
    state = { intakes = "u16", x = "u8", y = "u8" },
    hooks = {
        navi_intake = function(_side: number, navi: Object)
            local s = system.state() :: WatcherState
            s.intakes += 1
            s.x, s.y = navi.panel_x, navi.panel_y
        end,
        chip_check = function(_side: number, _navi: Object, chip: Chip?): Chip?
            if chip == test_chips.bomb then
                return test_chips.seed
            end
            return nil
        end,
    },
}

-- BN6's own Beast Out turns (content/bn6/rules/beast), which the test
-- navis' forms go by as BN6's do, its emotions (content/bn6/rules/
-- emotion: Full Synchro on a counter) and its dark chips' bug frags
-- (content/bn6/rules/dark-chips).
local beast = require("./beast")
local dark_chips = require("./dark-chips")
local emotion = require("./emotion")
local forms = require("./forms")

return {
    counter = counter,
    marker = marker,
    -- The test content's own rules: its systems, its rule sections (BN6's
    -- berserk, buster and math; the others are its Rust tables) and its
    -- roles.
    stock = define.ruleset {
        id = "stock",
        stock = true,
        systems = { beast, counter, forms.system, emotion.system, dark_chips },
        berserk = require("./berserk"),
        buster = require("./buster"),
        math = require("./math"),
        roles = require("./roles"),
    },
    -- Another player's: the marker first, then the counter.
    other = define.ruleset { id = "test-other", systems = { marker, counter } },
    -- BN6's patch cards, then the counter.
    cards = define.ruleset { id = "test-cards", systems = { patch_cards, counter } },
    -- The watcher.
    watch = define.ruleset { id = "test-watch", systems = { watcher } },
}
