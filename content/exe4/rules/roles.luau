--!strict
-- What EXE4's rules start, spawn, show and play by role (docs/design/content-model-v2.md §7.4, docs/design/
-- rules-in-luau.md R2: a game's roles are its root's). So far the roles that name only assets and data EXE4 has
-- ported: the statuses its damage word and counters give, its hit sparks, its navis' collision types, the anchor
-- region (EXE6's), and the banners its pack names (content/exe4/compat/assets.toml). The other roles wait
-- for EXE4's definitions (docs/design/exe4-map.md §18); the content check lists them as unfilled, and a battle stops at
-- the first it needs.

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
    sounds = {
        appear = asset.sound("appear"),
        custom_open = asset.sound("custom-open"),
    },
    music = {
        link_battle = asset.sound("netbattle"),
    },
    banners = {
        round_start = asset.banner("battle-start"),
        turn_start = asset.banner("turn-start"),
        win = asset.banner("enemy-deleted"),
    },
}

return roles
