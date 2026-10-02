--!strict
-- The test content's rules (docs/design/rules-in-luau.md): two made-up
-- systems and two rulesets, for the framework's tests of per-side state,
-- player setups and hooks. Neither changes how a battle plays. A third
-- ruleset plays by BN6's patch-cards system, for its tests.

local patch_cards = require("./patch-cards/system")

export type CounterState = { starts: number, side: number, bonus: number }

-- Counts the round starts it sees, and keeps its side and twice its
-- player's setup bonus.
local counter = define.system {
    id = "test/counter",
    state = { starts = "u8", side = "u8", bonus = "u16" },
    setup = { bonus = "u8" },
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

-- BN6's own Beast Out turns (content/bn6/rules/beast), which the test
-- navis' forms go by as BN6's do.
local beast = require("./beast/system")
local forms = require("./forms/system")

return {
    counter = counter,
    marker = marker,
    -- The test content's own rules.
    stock = define.ruleset { id = "test", stock = true, systems = { beast, counter, forms.system } },
    -- Another player's: the marker first, then the counter.
    other = define.ruleset { id = "test-other", systems = { marker, counter } },
    -- BN6's patch cards, then the counter.
    cards = define.ruleset { id = "test-cards", systems = { patch_cards, counter } },
}
