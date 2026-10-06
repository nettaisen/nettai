--!strict
-- EXE6's rules, their one definition (docs/design/rules-in-luau.md §2.5): what a side keeps through a round, what a
-- player brings, the hooks the framework calls (each says which of the game's modules it runs, in what order), the
-- custom screen's buttons and windows, EXE6's fields on its definitions, its rule sections and its roles. The modules
-- of this folder are the game's code (the save, the Crosses, Beast Out, the form changes, the NaviCust, the patch
-- cards, the emotions, the folder rules, the dark chips), which these call. The framework (Rust) still runs the EXE6
-- rules no module has taken over yet; the modules take them slice by slice (§8).

local beast = require("@self/beast")
local cross = require("@self/cross")
local dark_chips = require("@self/dark_chips")
local emotion = require("@self/emotion")
local folder = require("@self/folder")
local forms = require("@self/forms")
local fresh_stats = require("@self/fresh_stats")
local navicust = require("@self/navicust")
local patch_cards = require("@self/patch_cards")
local save = require("@self/save")

return define.rules {
    -- What a side keeps through a round.
    state = {
        -- Beast Out (./beast). The rush's (./beast/rush.luau).
        rush_phase = "u8",
        rush_started = "bool",
        rush_home_x = "u8",
        rush_home_y = "u8",
        rush_chain = "bool",
        rush_chain_block = "u8",
        rush_countdown = "u16",
        -- The controllers' (./beast/berserk.luau, ./beast/cross_special.luau), which share them as the original's
        -- share their 0x10 bytes.
        controller_step = "u8",
        controller_moves = "u16",
        controller_volley = "u16",
        controller_started = "bool",
        -- AIData+0x0F: the turn-start Beast Out check's delay.
        check_delay = "u8",
        -- Beast Out on the custom screen (./beast/custom.luau).
        beast_out_used = "bool",
        -- The Crosses (./cross): consecutive screens opened in ChargeCross (`dword_20349A0`), and the Cross
        -- window's (./cross/window.luau).
        charge_cross_screens = "u8",
        crosses_used = "bool[5]",
        offered = "u8[5]",
        offered_count = "u8",
        marked = "bool[5]",
        window_cursor = "u8",
        cross_chosen = "bool",
        chosen = "u8",
        -- What a form change notes of the round (./forms, `byte_203EAE0`): the navi went Beast Out (+2), crossed
        -- (+0xB).
        went_beast_out = "bool",
        crossed = "bool",
        -- The NaviCust's emotion-swing bug (./emotion): AIData+0x3A, ticks toward the next swing, and AIData+0x0B,
        -- the emotion it last rolled.
        swing_ticks = "u16",
        swung = "u8",
        -- The bug frags (./dark_chips, `dword_203F7E0`), which the round starts with the player's.
        bug_frags = "u32",
    },
    -- What a player brings (nettai-match from a match's side, exe6-compat from a save or a recording): a side's
    -- facts.
    setup = {
        -- The game's version: its Beast, its pictures, the Crosses it owns. No default: a side states it, or the
        -- round doesn't start. The versions come in the original's order (Gregar 0, Falzar 1: the number a navi's
        -- stats carry, NaviStats+0x20), which tools go by: they list and draw the versions in it, and number them
        -- by it.
        version = { "gregar", "falzar" },
        -- The Crosses they have (./cross/window.luau), up to five, of either version, in the window's order. A side
        -- that leaves them out has none. A save's are those of its version's five it owns (by Cross number:
        -- Gregar's event flags 0xE2-0xE6, Falzar's 0xE7-0xEB), which a boundary writes as the list of them.
        crosses = "form[5]",
        -- Beast Out unlocked (event flag 0xE0).
        beast_out = "bool",
        -- What the save brings to the navi's stats (./save): the base HP (HP Memories, +0x3E), the Regular memory
        -- (RegUps, +0x09), fighting in the sun (+0x22), and the level of the navi code it received (0 to 14, none
        -- without one: event flag 0x163, `dword_203CFA0`).
        hp = "u16",
        reg_up = "u8",
        sun = "bool",
        level = "u8?",
        -- The save's bug frags (./dark_chips), which the init exchange shares.
        bug_frags = "u32",
    },
    -- A setup that says nothing: Beast Out (a finished game's save); a new game's base HP, 100 (MegaMan's fresh
    -- HP), and Regular memory (the fresh stats'); no sun; no navi code received (no level).
    setup_defaults = { beast_out = true, hp = 100, reg_up = fresh_stats.reg_up, sun = false },
    -- EXE6's fields on its definitions (docs/design/rules-in-luau.md §7.5), which the engine checks and reads none
    -- of.
    extends = {
        chip = {
            -- A chip's Beast rush (./beast): the lock-on mode it runs by (none: where the navi stands), and
            -- `rush = false` for a chip with a mode that doesn't rush.
            beast = { lockon = "record:lockon", rush = "bool" },
            -- A dark chip (./dark_chips): what its user gets with no bug frag left (`off_8010D84`), and how much its
            -- use worsens the user's HP bug (`sub_800B79A`).
            dark_substitute = "chip",
            hp_bug = "u8",
        },
        form = {
            -- Its kind (./forms/kind.luau: every form but the base form names one), the navi a Cross (and one in
            -- Beast Out) is made with, and the animation a change into a Cross lets the navi go of (`sub_8014B18`:
            -- GroundCross's drill). (Whose game's form it is is the form's `version`, the engine's field.)
            kind = { "cross", "beast", "cross_beast", "beast_over" },
            cross_of = "navi",
            cross_release_anim = "u8",
            -- ChargeCross's extra chips (`extra_chips`, `sub_802A49C`) and DustCross's scrap button
            -- (`scrap_button`, `sub_8027F10`), and theirs in Beast Out (./cross).
            extra_chips = "bool",
            scrap_button = "bool",
            -- A Cross's form in Beast Out, the Cross special's buster volley in a form (`special_volley`,
            -- `sub_802D4F0`), and a form whose charged sword runs inside the rush (./beast).
            beast = "form",
            special_volley = "u16",
            charged_sword_rush = "bool",
        },
    },
    -- Its own actions, which reach its state: the change into a form (./forms), the Beast rush and the Cross
    -- special (./beast).
    actions = { forms.action, beast.rush_action, beast.cross_special_action },
    -- The custom screen's buttons: DustCross's scrap (./cross), Beast Out (./beast/custom.luau), ChpShufl's re-deal
    -- (./navicust).
    buttons = {
        scrap = cross.scrap,
        beast_out = beast.custom.button,
        redeal = navicust.redeal,
    },
    -- Its windows: the Cross window's four (./cross/window.luau) and Beast Out's two.
    windows = {
        cross_opening = cross.window.opening,
        cross_window = cross.window.window,
        cross_closing = cross.window.closing,
        cross_chosen = cross.window.chosen,
        beast_out = beast.custom.window,
        beast_out_chip = beast.custom.chip_window,
    },
    custom = {
        -- ChargeCross's extra chips, NumbrOpn's 10, the hand-shrink bug.
        hand_size = cross.hand_size,
        open = function(side: number)
            cross.window.open(side)
            beast.custom.open(side)
        end,
        confirmed = function(side: number)
            cross.window.confirmed(side)
            beast.custom.confirmed(side)
        end,
        keys = cross.window.keys,
        take_back = cross.window.take_back,
        chip_picked = beast.custom.chip_picked,
        chip_taken_back = beast.custom.chip_taken_back,
    },
    hooks = {
        -- What the save brings to the navi's stats, and a link navi's at its level; then the NaviCust's compile,
        -- which reads them; then the patch cards, which apply to what it made; then the round's bug frags.
        round_setup = function(side: number)
            save.round_setup(side)
            navicust.round_setup(side)
            patch_cards.apply(side)
            dark_chips.round_setup(side)
        end,
        -- Tired at the start, and the NaviCust's swing bug (./emotion).
        round_start = emotion.round_start,
        countered = emotion.countered,
        navi_tick = emotion.navi_tick,
        -- Beast Out's turns, its rush, Beast Over's berserk and the Cross special (./beast).
        turn_check = beast.turn_check,
        turn_started = beast.turn_started,
        custom_requested = beast.custom_requested,
        custom_closed = beast.custom_closed,
        chip_used = beast.chip_used,
        controller = beast.controller,
        takeover_requested = beast.takeover_requested,
        takeover = beast.takeover,
        form_reverted = beast.form_reverted,
        -- A dark chip's bug frag, its substitute, and the HP bug its use worsens (./dark_chips).
        chip_substitute = dark_chips.chip_substitute,
        chip_prepared = dark_chips.chip_prepared,
        -- The folder's rules, which tools ask (the simulation never does).
        folder_check = folder.check,
    },
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    banners = require("@self/banners"),
    berserk = require("@self/berserk"),
    buster = require("@self/buster"),
    chip_use = require("@self/chip_use"),
    custom_screen = require("@self/custom_screen"),
    effects = require("@self/effects"),
    elements = require("@self/elements"),
    flow = require("@self/flow"),
    fresh_stats = fresh_stats,
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
}
