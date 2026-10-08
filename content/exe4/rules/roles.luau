--!strict
-- What EXE4's rules start, spawn, show and play by role (docs/design/content-model-v2.md §7.4, docs/design/
-- rules-in-luau.md R2: a game's roles are its root's). So far the roles that name only assets and data EXE4 has
-- ported: the statuses its damage word and counters give, its hit sparks, its navis' collision types, the anchor
-- region (EXE6's), and the banners its pack names (content/exe4/compat/assets.toml). The other roles wait
-- for EXE4's definitions (docs/design/exe4-map.md §18); the content check lists them as unfilled, and a battle stops at
-- the first it needs.

local anti_damage = require("../chips/anti/counter")
local anti_recovery = require("../chips/anti/recovery")
local anti_sword = require("../chips/anti/sword")
local barriers = require("../lib/barriers")
local charge_glow = require("../objects/charge_glow")
local collision = require("./collision")
local deletion = require("./deletion")
local ice_cube = require("../objects/ice_cube")
local invalid = require("../chips/invalid").invalid
local machine_gun = require("../navis/megaman/forms/gutssoul/machine_gun")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")
local statuses = require("./status")
local support_chips = require("../chips/supports")
local supports = require("../lib/supports/controller")
local underground_hole = require("../objects/underground_hole")
local zeroed = require("../chips/zeroed").zeroed

local roles: RolesSpec = {
    chips = {
        -- What a zeroed chip field reads (EXE4's chip 0: chips/zeroed).
        zeroed = zeroed,
        -- What a pick the custom screen refuses counts as (EXE4's chip 0x184, past the named ones: chips/invalid).
        invalid = invalid,
        -- The chips the NaviCust supports' telops name (EXE4's 0x17F to 0x181: chips/supports).
        rush = support_chips.rush,
        beat = support_chips.beat,
        tango = support_chips.tango,
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
        -- AntiDmg's counter (action 0x38, 0x080ED972), which its trap's catch starts.
        anti_damage_counter = anti_damage.action,
        -- AntiSwrd's counter (action 0x39, 0x080EDA68), which its trap's catch starts.
        anti_sword_counter = anti_sword.action,
        -- What request 0x20 starts from idle (action 0x10, 0x080EEBEA): GutsSoul's machine gun, which its rapid
        -- presses ask for.
        forced_charged_shot = machine_gun.action,
    },
    kinds = {
        -- The glow a navi's charge brings (effect object 5, 0x080E215C; the navi's tail spawns it, 0x0800BD88).
        charge_glow = charge_glow.kind,
        -- AntiRecv's counterattack (effect object 0x2C, 0x080E538C), which a recovery springs (0x080E5436).
        anti_recovery = anti_recovery.kind,
        -- The NaviCust supports' controller (Rush's, Beat's and Tango's: effect object 0x74, lib/supports).
        support = supports.kind,
        -- The hole over a navi under the ground (effect object 0x3E, 0x080E64FC), which the navi's tail keeps
        -- (0x0800C0A6).
        dive_ripple = underground_hole.kind,
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
        -- A trap springs (the trap's mark, 0x0800815C: 0x10D).
        cut_in = asset.sound("dimming-sparkle"),
    },
    effects = {
        -- HP recovered (the heal, 0x0800ABCA: effect row 7, 0x080E16B4).
        recovery = new.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 },
        -- A sprung trap's mark (0x0800815C: effect row 0x48, 14-05's animation 7): AntiNavi's over the turned chip.
        trap_mark = new.effect { sprite = asset.sprite("hit-sparks"), anim = 7, palette = 0 },
        -- What an obstacle encased in ice or a bubble flickers with (0x08011764: effect row 0x6E, the rock sheet's
        -- ice block, 10-02's animation 2).
        encased = new.effect { sprite = asset.sprite("rock"), anim = 2, palette = 0 },
    },
    hooks = {
        -- The first barrier (the navi's init, 0x0800D892: NaviStats +0x21, FstBarr's and the patch cards' aura): the
        -- navi enters with the barrier its stats name, and its visual (../lib/barriers).
        first_barrier = function(navi: Object)
            local barrier = battle.navi(navi.alliance).first_barrier
            if barrier then
                barriers.raise(navi, barrier, true)
            end
        end,
        -- All Guard (0x0800D8E4: NaviStats +0x28 at 1, the rules' stat `all_guard`): as the navi's abilities reset
        -- (its init, 0x0800D8C2), the guard the reset took down goes up again: every hit that doesn't break guards is
        -- turned aside (0x08012B84).
        abilities_reset = function(navi: Object)
            if battle.navi(navi.alliance).all_guard then
                navi:set_status("guard", true)
            end
        end,
        -- What an obstacle encased in ice or a bubble becomes (0x08011788: an ice block or the ice cube; EXE4's
        -- encasing is ColdMan's, chips/coldman).
        encased = ice_cube.replace,
    },
    sprites = {
        -- The Full Synchro aura (actor object 0x5E, 0x080CD1A8: 14-16, EXE5's and EXE6's numbers too).
        full_synchro_aura = asset.sprite("full-synchro-aura"),
        -- The status visuals over a navi (effect 6's rows, 0x080E22B8: 14-0B, 14-09; it has no third row, and the status
        -- routine spawns none for the immobilized).
        confusion = asset.sprite("confusion"),
        blindness = asset.sprite("blindness"),
        -- The mark over a navi a weakness hit damaged (effect 0x6B, 0x080E9900: 14-07, its animation 0 whatever the
        -- spawn's).
        hit_marker = asset.sprite("hit-marker"),
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
