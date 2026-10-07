--!strict
-- EXE4's rules, their one definition (docs/design/rules-in-luau.md §2.5): what a player brings, the hooks the framework
-- calls, the custom screen's buttons and windows, EXE4's fields on its definitions, its rule sections and its roles.
-- The port is under way (docs/design/exe4-map.md): a section's value the engine can't say EXE4's way yet is the nearest
-- it can, each one an entry of exe4-map.md §18, which lists what is still to port.

local dark_chips = require("@self/dark_chips")
local folder = require("@self/folder")
local fresh_stats = require("@self/fresh_stats")
local light_dark = require("@self/light_dark")
local patch_cards = require("@self/patch_cards")
local navicust = require("@self/navicust")
local save = require("@self/save")
local souls = require("@self/souls")

-- What a side keeps through a round: the dark chips' (./dark_chips) and Double Soul's (./souls).
local state: StateSchema = {}
for _, part in { dark_chips.state, souls.state } do
    for name, field in part do
        state[name] = field
    end
end

local rules: Rules = {
    -- EXE4's own stats (NaviStats bytes the engine's stats don't hold): BustPack's weapon level (+0x0B, which the
    -- charged shot reads), and the NaviCust's and the patch cards' move bug (+0x0D: 0xFF confused at the start, 0x10
    -- and 0x20 steps of its own, right and left: ./navicust).
    stats = {
        weapon_level = "u8",
        move_bug = "u8",
        -- The Full Synchro at the start (+0x1F: patch card 45's, ./patch_cards; the reload's reset clears it), which
        -- the starting mood reads (./light_dark).
        full_synchro_start = "bool",
        -- MegaMan's color (+0x27: 0, or patch cards 59, 60, 89, 90's 1 to 4, ./patch_cards; the reset clears it),
        -- which his palette reads (./light_dark).
        color = "u8",
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
        -- MegaMan's light/dark value (NaviStats +0x36): ./light_dark.
        karma = "u16",
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
        -- The game's version: the souls it can have (./souls). No default: a side states it, or the round doesn't
        -- start. The versions come in the original's order (Red Sun, then Blue Moon), which tools go by.
        version = { "redsun", "bluemoon" },
        -- The souls the save has (./souls/custom), of its version's six (a save's soul flags, 0x08020018's by soul:
        -- Red Sun's event flags 0x17 to 0x1C, Blue Moon's 0x1D to 0x22), which a boundary writes as the list of them;
        -- the engine's form list. A side that leaves them out has none. And Double Soul (event flag 0x14): without it
        -- the custom screen has no UNITE button.
        souls = schema.role("form_list", "form[6]"),
        double_soul = "bool",
    },
    -- A setup that says nothing: what every save brings at its highest; the default block's light/dark value (500:
    -- the starting mood 0x99); Double Soul (a finished game's save).
    setup_defaults = {
        hp = save.MAX_HP,
        reg_up = save.MAX_REG_UP,
        karma = 500,
        double_soul = true,
    },
    state = state,
    -- EXE4's fields on its definitions (docs/design/rules-in-luau.md §7.5), which the engine checks and reads none of.
    -- What a soul's form says for Double Soul (./souls; a soul is named by its form's id, the original's number for it,
    -- NaviStats +0x24, is compat's): the family of the chips given up for it (0x08020008's table) and its turns
    -- (0x080180A8's: 3 every one), its chip (which OK puts in the hand for it, its use the soul's change:
    -- ./souls/change), and its navi's image at the change (./souls/image, a `SoulImage`).
    extends = {
        form = {
            soul = {
                family = {
                    "recovery", "break", "wind", "invisible", "fire", "elec", "sword", "plus", "metal", "summon",
                    "aqua", "wood",
                },
                turns = "u8",
            },
            chip = "chip",
            image = "record:soul-image",
        },
    },
    -- Its own actions, which reach its state: the souls' changes and the revert (./souls).
    actions = souls.actions,
    -- The custom screen's buttons and windows: Double Soul's (./souls/custom).
    buttons = souls.custom.buttons,
    windows = souls.custom.windows,
    -- The custom screen's: the hand's size (./souls/custom); the dark chips' offer as it deals and what is sent of them
    -- (./dark_chips); Double Soul's offer as the screen opens and the soul given at OK (./souls/custom).
    custom = {
        hand_size = souls.custom.hand_size,
        open = souls.custom.open,
        deal = dark_chips.deal,
        chip_picked = dark_chips.chip_picked,
        chip_taken_back = dark_chips.chip_taken_back,
        confirmed = function(side: number)
            dark_chips.confirmed(side)
            souls.custom.confirmed(side)
        end,
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
        -- dark one clears and the hole a navi that isn't dark closes; then Double Soul's change or revert asked for
        -- (./souls, 0x0800B658).
        starting_mood = light_dark.starting_mood,
        navi_intake = function(side: number, navi: Object)
            light_dark.navi_intake(side, navi)
            souls.navi_intake(side, navi)
        end,
        -- A soul's turns, counted as a custom screen is asked for (./souls).
        custom_requested = souls.custom_requested,
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
            folder.validate(side, problem)
            return out
        end,
        -- What rules a folder a tool asks of breaks (./folder).
        folder_check = folder.check,
    },
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    banners = require("@self/banners"),
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
