--!strict
-- MegaMan: the navi that changes form.

local heatcross = require("./forms/heatcross/form")
local eleccross = require("./forms/eleccross/form")
local slashcross = require("./forms/slashcross/form")
local erasecross = require("./forms/erasecross/form")
local chargecross = require("./forms/chargecross/form")
local spoutcross = require("./forms/spoutcross/form")
local tomahawkcross = require("./forms/tomahawkcross/form")
local tengucross = require("./forms/tengucross/form")
local groundcross = require("./forms/groundcross/form")
local dustcross = require("./forms/dustcross/form")
local gregar_beast = require("./forms/gregar-beast/form")
local falzar_beast = require("./forms/falzar-beast/form")
local gregar_beast_over = require("./forms/gregar-beast-over/form")
local falzar_beast_over = require("./forms/falzar-beast-over/form")
local buster = require("./weapons/buster/weapon")
local charged_shot = require("./weapons/charged-shot/weapon")

return define.navi {
    id = "megaman",
    sprite = asset.sprite("megaman-navi"),
    element = "null",
    buster_bonus = 1,
    move_lag = { 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4 },
    banners = { win = asset.banner("megaman-win"), lose = asset.banner("megaman-deleted") },
    merge_height = 0,
    -- The no-running message's portrait: the speaker beside the box. Its
    -- words are locales/<lang>.toml's; the content's own (en) time it.
    run_message = {
        portrait = asset.sprite("megaman-portrait"),
    },
    -- What a navi switch brings the navi with (`byte_80210DD`): its weapons,
    -- and the rest of its fresh stats; and its HP after the change, by side
    -- (`byte_802DD88`).
    weapons = { buster = buster.weapon, charge_shot = charged_shot.weapon },
    fresh = { hp = 100, mega_level = 5, giga_level = 1 },
    cross_hp = { 999, 999 },
    -- What his link navi level adds over his NaviCust (a navi code of his
    -- received over the link cable; `pt_8121200`'s scripts,
    -- docs/engine/link-navis.md). His row of `off_8120F44` is zeros: what
    -- the reload would give him, were it to run for him with a level.
    levels = {
        base_hp = { 0, 0, 0, 0, 0, 0, 0 },
        by_level = {
            { hp = 20 },
            { hp = 40 },
            { hp = 60, attack = 1 },
            { hp = 80, attack = 1 },
            { hp = 100, attack = 1, rapid = 1 },
            { hp = 120, attack = 1, rapid = 1 },
            { hp = 140, attack = 1, rapid = 1, charge = 1 },
            { hp = 160, attack = 1, rapid = 1, charge = 1 },
            { hp = 180, attack = 1, rapid = 1, charge = 1 },
            { hp = 200, attack = 1, rapid = 2, charge = 1, custom_level = 1, mega_level = 1 },
            { hp = 220, attack = 2, rapid = 2, charge = 2, custom_level = 1, mega_level = 1 },
            { hp = 240, attack = 2, rapid = 2, charge = 2, custom_level = 1, mega_level = 1 },
            { hp = 260, attack = 2, rapid = 3, charge = 2, custom_level = 2, mega_level = 1 },
            { hp = 280, attack = 2, rapid = 3, charge = 3, custom_level = 2, mega_level = 2 },
            { hp = 300, attack = 3, rapid = 3, charge = 3, custom_level = 2, mega_level = 2 },
        },
    },
    -- His forms, by game: the Crosses by their number on the custom screen, Beast Out and
    -- Beast Over.
    forms = {
        gregar = { crosses = { heatcross, eleccross, slashcross, erasecross, chargecross }, beast_out = gregar_beast, beast_over = gregar_beast_over },
        falzar = { crosses = { spoutcross, tomahawkcross, tengucross, groundcross, dustcross }, beast_out = falzar_beast, beast_over = falzar_beast_over },
    },
    identity = define.identity {
        class = "megaman",
        -- Where a held thing with no attach point sits (BatCan's cannon).
        held_offset = { 0, 0 },
        version = 0,
        actor_type = "player",
        ai_index = 0,
        -- The Full Synchro aura's animation, and the ice block that fits.
        aura_anim = 0,
        ice = "medium",
        -- He wears what his form does: his death hook takes it off, and an animation
        -- change, a flinch and a drag restart it.
        overlay_hooks = { "death", "refresh", "flinch", "drag" },
        attach_points = {
            { 2, 0 },
            { 2, 0 },
            { 2, 0 },
            { 8, 48 },
            { 8, 48 },
            { 8, 48 },
            { 16, 0 },
            { 16, 24 },
            { 16, 24 },
            { 24, 24 },
            { 18, 24 },
            { 18, 24 },
            { 18, 24 },
            { 24, 24 },
            { 23, 25 },
            { 24, 24 },
            { 40, 26 },
            { 5, 11 },
            { 31, 25 },
            { 40, 26 },
            { 20, 25 },
            { 2, 16 },
            { 24, 24 },
            { 24, 25 },
            { 24, 23 },
            { 22, 25 },
            { 28, 25 },
            { 2, 18 },
            { 88, 16 },
            { 120, -4 },
            { -8, 16 },
            { 64, 16 },
            { 2, 0 },
            { 0, 0 },
        },
    },
}
