--!strict
-- EXE4's rules, their one definition (docs/design/rules-in-luau.md §2.5): what a player brings, the hooks the framework
-- calls, its rule sections and its roles. The port is under way (docs/design/exe4-map.md): so far a netbattle of
-- MegaMan with his buster and charged shot; a section's value the engine can't say EXE4's way yet is the nearest it can,
-- each one an entry of exe4-map.md §18, which lists what is still to port (the NaviCust, the Mod Cards, the souls, the
-- custom screen's own states, the chips).

local fresh_stats = require("@self/fresh_stats")
local save = require("@self/save")

local rules: Rules = {
    -- What a player brings (nettai-match from a match's side, exe4-compat from a recording): a side's facts.
    setup = {
        -- The navi they play: the round's stats are its. No default: a side states it.
        navi = "navi",
        -- Their folder as the save keeps it, an entry with no chip empty (a folder being made); the Regular chip, an
        -- entry of it (from 0), dealt first.
        folder = schema.list({ chip = "chip", code = "code" }, 30),
        regular_chip = "u8?",
        -- What the save brings to the navi's stats (./save): MegaMan's HP and the Regular memory.
        hp = "u16",
        reg_up = "u8",
    },
    -- A setup that says nothing: what every save brings at its highest.
    setup_defaults = {
        hp = save.MAX_HP,
        reg_up = save.MAX_REG_UP,
    },
    hooks = {
        -- What the save brings to the navi's stats (./save).
        round_setup = save.round_setup,
        -- What is wrong with a side's setup, which tools ask (the simulation never does).
        validate = function(side: number): { SetupProblem }
            local out: { SetupProblem } = {}
            save.validate(side, function(text: string, field: string?, entry: number?)
                table.insert(out, { text = text, field = field, entry = entry })
            end)
            return out
        end,
    },
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    buster = require("@self/buster"),
    chip_use = require("@self/chip_use"),
    effects = require("@self/effects"),
    elements = require("@self/elements"),
    flow = require("@self/flow"),
    fresh_stats = fresh_stats,
    link_pick = require("@self/link_pick"),
    math = require("@self/math"),
    panels = require("@self/panels"),
    pools = require("@self/pools"),
    reactions = require("@self/reactions"),
    roles = require("@self/roles"),
    status = require("@self/status").rules,
}

return rules
