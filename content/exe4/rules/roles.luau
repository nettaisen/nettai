--!strict
-- What EXE4's rules start, spawn, show and play by role (docs/design/content-model-v2.md §7.4, docs/design/
-- rules-in-luau.md R2: a game's roles are its root's). So far the roles that name only assets and data EXE4 has
-- ported: the statuses its damage word and counters give, its hit sparks, its navis' collision types, the anchor
-- region (EXE6's), and the banners its pack names (content/exe4/compat/assets.toml). The other roles wait
-- for EXE4's definitions (docs/design/exe4-map.md §18); the content check lists them as unfilled, and a battle stops at
-- the first it needs.

local charge_glow = require("../objects/charge_glow")
local collision = require("./collision")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("./status")

local roles: RolesSpec = {
    statuses = {
        -- What a damage word's paralysis bit makes its hits carry (0x08012860: status byte 0x12).
        damage_word_paralysis = statuses.paralyze_150,
    },
    sparks = {
        plain = sparks.plain,
        guard = sparks.guard,
    },
    collision = {
        navi = collision.navi,
        floating_navi = collision.floating_navi,
        navi_target = collision.navi_target,
    },
    regions = {
        anchor = regions.single,
    },
    kinds = {
        -- The glow a navi's charge brings (effect object 5, 0x080E215C; the navi's tail spawns it, 0x0800BD88).
        charge_glow = charge_glow.kind,
    },
    sounds = {
        appear = asset.sound("appear"),
        -- What the custom screen can't pick or take back (0x08020386, 0x08020634).
        refused = asset.sound("refused"),
        -- The custom screen's, which its player hears (docs/design/exe4-map.md §5): the window slides in (0x0801E24E);
        -- the cursor moves (0x0802049C); a pick, OK, a pick taken back (0x080203A0 by 0x080204B8, 0x0802063E); R's
        -- description opens (0x080203C0; its close, SELECT's hiding and L's message play none); a dark chip under the
        -- cursor, every 61 ticks (0x0801E4A6); the Program Advance animation's chips of the recipe and the Program
        -- Advance (0x0801E74E, 0x0801E83A).
        custom_open = asset.sound("custom-open"),
        custom_cursor = asset.sound("custom-cursor"),
        custom_pick = asset.sound("custom-pick"),
        custom_ok = asset.sound("custom-ok"),
        custom_back = asset.sound("custom-back"),
        custom_description = asset.sound("description-open"),
        custom_shade = asset.sound("dark-hover"),
        program_advance_part = asset.sound("program-advance-part"),
        program_advance = asset.sound("program-advance"),
        -- The fight's: a navi's hit, every console's (0x08013A96: 0x6B), a navi in auto battle's (0x6D).
        hit = asset.sound("hit"),
        auto_battle_hit = asset.sound("auto-battle-hit"),
    },
    music = {
        link_battle = asset.sound("netbattle"),
    },
    banners = {
        round_start = asset.banner("battle-number-start"),
        turn_start = asset.banner("turn-start"),
        win = asset.banner("enemy-deleted"),
    },
}

return roles
