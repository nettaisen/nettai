--!strict
-- What EXE4's rules start, spawn, show and play by role (docs/design/content-model-v2.md §7.4, docs/design/
-- rules-in-luau.md R2: a game's roles are its root's). So far the roles that name only assets and data EXE4 has
-- ported: the statuses its damage word and counters give, its hit sparks, its navis' collision types, the anchor
-- region (EXE6's), and the banners its pack names (content/exe4/compat/assets.toml). The other roles wait
-- for EXE4's definitions (docs/design/exe4-map.md §18); the content check lists them as unfilled, and a battle stops at
-- the first it needs.

local barriers = require("../lib/barriers")
local charge_glow = require("../objects/charge_glow")
local collision = require("./collision")
local deletion = require("./deletion")
local invalid = require("../chips/invalid").invalid
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("./status")

local roles: RolesSpec = {
    chips = {
        -- What a pick the custom screen refuses counts as (EXE4's chip 0x184, past the named ones: chips/invalid).
        invalid = invalid,
    },
    statuses = {
        -- What a damage word's paralysis bit makes its hits carry (0x08012860: status byte 0x12); a counter hit's
        -- paralysis, the same (0x0800B08E: the hit flags' 0x40 sets status byte 0x12).
        damage_word_paralysis = statuses.paralyze_150,
        counter_paralysis = statuses.paralyze_150,
    },
    sparks = {
        plain = sparks.plain,
        guard = sparks.guard,
        -- Lava's burn (0x08013128 and 0x0801309E: hit effect 1, jittered by 15, from 0x080E2140).
        panel_burn = sparks.fire,
    },
    collision = {
        navi = collision.navi,
        floating_navi = collision.floating_navi,
        navi_target = collision.navi_target,
    },
    regions = {
        anchor = regions.single,
    },
    actions = {
        -- A player's deletion (action 2, 0x08010850).
        deletion = deletion.action,
    },
    kinds = {
        -- The glow a navi's charge brings (effect object 5, 0x080E215C; the navi's tail spawns it, 0x0800BD88).
        charge_glow = charge_glow.kind,
    },
    sounds = {
        appear = asset.sound("appear"),
        -- The confusion's visual, every 60 ticks (effect 6, 0x080E2354).
        confusion = asset.sound("confusion"),
        -- An obstacle taking damage (0x08014058: the stages' boulder).
        damage = asset.sound("damage"),
        -- The custom gauge fills (0x080147F4); the HP box's alarm (0x0801475E).
        gauge_full = asset.sound("gauge-full"),
        low_hp = asset.sound("low-hp"),
        -- A counter hit (0x08013410); a chip's Full Synchro boost (0x0800D54E).
        counter_hit = asset.sound("counter-hit"),
        damage_bonus = asset.sound("damage-bonus"),
        -- A panel cracks or breaks (0x08009AEC); a panel turns to poison (0x08009D68).
        panel_crack = asset.sound("panel-crack"),
        panel_poison = asset.sound("panel-poison"),
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
        -- A hit turned aside (the guard's spark, 0x0800B0F6: 0x6E). Unlike EXE6's, the routine doesn't test for a
        -- pause: a guarded object hit the tick before a pause and running through it would sound it again, which
        -- nothing of EXE4's content does (the stages' gear runs while paused only before the fight).
        guard = asset.sound("guard"),
        -- HP recovered (the heal, 0x0800ABCA: 0x9A).
        recovery = asset.sound("heal"),
    },
    effects = {
        -- HP recovered (the heal, 0x0800ABCA: effect row 7, 0x080E16B4).
        recovery = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
    },
    hooks = {
        -- The first barrier (the navi's init, 0x0800D892: NaviStats +0x21, FstBarr's and the patch cards' aura): the
        -- navi enters with the barrier its stats name, and its visual (../lib/barriers).
        first_barrier = function(navi: Object)
            local barrier = battle.navi(navi.alliance).first_barrier
            if barrier then
                barriers.raise(navi, barrier)
            end
        end,
    },
    sprites = {
        -- The Full Synchro aura (actor object 0x5E, 0x080CD1A8: 14-16, EXE5's and EXE6's numbers too).
        full_synchro_aura = asset.sprite("full-synchro-aura"),
        -- The status visuals over a navi (effect 6's rows, 0x080E22B8: 14-0B, 14-09; it has no third row, and the status
        -- routine spawns none for the immobilized).
        confusion = asset.sprite("confusion"),
        blindness = asset.sprite("blindness"),
    },
    music = {
        link_battle = asset.sound("netbattle"),
        -- The round's end (0x080071F8): the winner's, a special battle's (battle flag 2); a netbattle's loser's
        -- (0x0800727C).
        winner = asset.sound("winner"),
        winner_special = asset.sound("winner-special"),
        loser = asset.sound("loser"),
    },
    banners = {
        -- A round's start, a turn's (a netbattle's: 0x080070BA), the final turns' (0x08007094).
        round_start = asset.banner("battle-number-start"),
        turn_start = asset.banner("turn-start"),
        final_turn = asset.banner("final-turn"),
        -- A draw (0x0800730C); a win's, and a win and a loss on the judge's ruling (0x08007220, 0x080072C8); the
        -- damage judge's (0x080163EE).
        draw = asset.banner("draw"),
        win = asset.banner("enemy-deleted"),
        win_judged = asset.banner("you-win"),
        lose_judged = asset.banner("you-lose"),
        judge = asset.banner("hit-damage-judge"),
        -- A Program Advance, and a selection that makes none (0x0801E696: no netbattle's screen shows the second).
        program_advance = asset.banner("program-advance"),
        program_advance_empty = asset.banner("program-advance-empty"),
        -- (No `telop` nor `telop_remote`: a dimming chip's telop, 0x08008C2A, has no banner of its own. 0x0801650C lays
        -- the chip's name on the banner block, the user's console's, 0x08016454, or on the second block, the other
        -- player's, 0x080164B4, each in the banners' steps, and the HUD draws it in its own look, `Hud.layout.telop`.)
    },
}

return roles
