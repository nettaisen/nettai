--!strict
-- EXE4's rules, their one definition (docs/design/rules-in-luau.md §2.5): what a player brings, the hooks the framework
-- calls, its rule sections and its roles. The port is under way (docs/design/exe4-map.md): so far a netbattle of
-- MegaMan with his buster and charged shot; a section's value the engine can't say EXE4's way yet is the nearest it can,
-- each one an entry of exe4-map.md §18, which lists what is still to port (the NaviCust, the patch cards, the souls, the
-- custom screen's own states, the chips).

local custom = require("@self/custom")
local dark_chips = require("@self/dark_chips")
local fresh_stats = require("@self/fresh_stats")
local light_dark = require("@self/light_dark")
local patch_cards = require("@self/patch_cards")
local navicust = require("@self/navicust")
local save = require("@self/save")

local rules: Rules = {
    -- EXE4's own stats (NaviStats bytes the engine's stats don't hold): BustPack's weapon level (+0x0B, which the
    -- charged shot reads), and the NaviCust's and the patch cards' move bug (+0x0D: 0xFF confused at the start, 0x10
    -- and 0x20 steps of its own, right and left: ./navicust).
    stats = {
        weapon_level = "u8",
        move_bug = "u8",
    },
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
        -- MegaMan's light/dark value (NaviStats +0x36) and the Full Synchro at the start (+0x1F): ./light_dark.
        karma = "u16",
        full_synchro_start = "bool",
        -- Their NaviCust (./navicust): the programs placed on its 5x5 board (the engine's 7x7 grid's middle) in its
        -- list's order, each in one of its colors (its definition's `colors`), its center at (x, y), turned
        -- `rotation` quarters clockwise, compressed or not.
        navicust_programs = schema.list({
            program = "navicust_programs",
            color = { "white", "pink", "yellow", "red", "blue", "green" },
            x = "u8",
            y = "u8",
            rotation = "u8",
            compressed = "bool",
        }, 25),
        -- Their patch cards (./patch_cards): each card switched on and the slot it is in (a save's 0x464C, by slot).
        patch_cards = schema.list({ card = "patch_cards", slot = "u8" }, patch_cards.SLOTS),
    },
    -- A setup that says nothing: what every save brings at its highest; the default block's light/dark value (500:
    -- the starting mood 0x99), no Full Synchro at the start.
    setup_defaults = {
        hp = save.MAX_HP,
        reg_up = save.MAX_REG_UP,
        karma = 500,
        full_synchro_start = false,
    },
    -- What a side keeps through a round: the dark chips' (./dark_chips).
    state = dark_chips.state,
    -- The custom screen's: the hand's size (./custom); the dark chips' offer as it deals and what is sent of them
    -- (./dark_chips).
    custom = {
        hand_size = custom.hand_size,
        deal = dark_chips.deal,
        chip_picked = dark_chips.chip_picked,
        chip_taken_back = dark_chips.chip_taken_back,
        confirmed = dark_chips.confirmed,
    },
    hooks = {
        -- What the save brings to the navi's stats (./save), then the NaviCust and the patch cards compiled into them
        -- (./navicust, ./patch_cards).
        round_setup = function(side: number)
            save.round_setup(side)
            navicust.run(side)
        end,
        -- As each hand is installed, its dark chips' costs (./dark_chips).
        custom_result = dark_chips.custom_result,
        -- Light and dark MegaMan (./light_dark): his starting mood; each tick of the navi's intake, the holy panel a
        -- dark one clears and the hole a navi that isn't dark closes.
        starting_mood = light_dark.starting_mood,
        navi_intake = light_dark.navi_intake,
        -- Light and dark MegaMan (./light_dark): his palette.
        navi_palette = light_dark.navi_palette,
        -- What is wrong with a side's setup, which tools ask (the simulation never does).
        validate = function(side: number): { SetupProblem }
            local out: { SetupProblem } = {}
            local function problem(text: string, field: string?, entry: number?)
                table.insert(out, { text = text, field = field, entry = entry })
            end
            save.validate(side, problem)
            navicust.validate(side, problem)
            patch_cards.validate(side, problem)
            return out
        end,
    },
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    buster = require("@self/buster"),
    chip_use = require("@self/chip_use"),
    custom_screen = require("@self/custom_screen"),
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
