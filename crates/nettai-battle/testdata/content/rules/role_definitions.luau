--!strict
-- The engine's test content's own effects, hit sparks, region, collision
-- types, sounds, music and banners for the rules' roles
-- (rules/roles.luau): made-up looks (one sprite for every effect, one for
-- every spark), collision types by what they are, and a made-up asset for
-- every sound, music and banner.

local sprites = require("../sprites")

local function effect(): Effect
    return new.effect { sprite = sprites["test-burst"], anim = 0, palette = 0, steps = 9 }
end

local function spark(): Spark
    return new.spark { sprite = sprites["test-spark"], anim = 0, palette = 0, steps = 4 }
end

-- A type of members `flags` (an own side's of the object's side, the
-- other side's mirrored). `row` is the row the type has in EXE6's table: a
-- bug code's high byte takes its offset.
local function collision(id: string, row: number, flags: { [CollisionMember]: boolean }): CollisionType
    return new.collision { id = "test/" .. id, flags = flags, row_offset = row * 8 }
end

-- A navi's body: its side's body and player navi, which resolves while the
-- battle is dimmed and reaches floating bodies.
local BODY: { [CollisionMember]: boolean } = {
    own_body = true,
    own_navi = true,
    hits_while_dimmed = true,
    reaches_float_shoe = true,
}
-- The other side's objects and bodies, and neutral objects.
local OTHERS: { [CollisionMember]: boolean } = {
    enemy_object = true,
    enemy_body = true,
    enemy_other_body = true,
    neutral = true,
}

return {
    effects = {
        deletion = effect(),
        recovery = effect(),
        cut_in_flash = effect(),
        trap_mark = effect(),
        encased = effect(),
        burst = effect(),
    },
    sparks = {
        plain = spark(),
        guard = spark(),
        thrown_obstacle = spark(),
    },
    regions = {
        anchor = new.region { panels = { { 0, 0 } } },
    },
    collision = {
        navi = collision("navi", 0x01, BODY),
        floating_navi = collision(
            "floating-navi",
            0x10,
            { own_body = true, own_navi = true, floating = true, hits_while_dimmed = true, reaches_float_shoe = true }
        ),
        navi_target = collision(
            "navi-target",
            0x02,
            { enemy_attack = true, enemy_object = true, enemy_body = true, enemy_other_body = true, neutral = true }
        ),
        thrown_obstacle = collision("thrown-obstacle", 0x06, { own_attack = true, reaches_float_shoe = true }),
        thrown_obstacle_target = collision("thrown-obstacle-target", 0x05, OTHERS),
    },
    -- Made-up sounds, music, sprites and banners, one for each role (named
    -- `test-<kind>-<role>`; the test content gives each the id the tests
    -- look for).
    sounds = {
        panel_crack = asset.sound("test-sound-panel-crack"),
        own_hit = asset.sound("test-sound-own-hit"),
        hit = asset.sound("test-sound-hit"),
        damage = asset.sound("test-sound-damage"),
        guard = asset.sound("test-sound-guard"),
        counter_hit = asset.sound("test-sound-counter-hit"),
        deleted = asset.sound("test-sound-deleted"),
        recovery = asset.sound("test-sound-recovery"),
        damage_bonus = asset.sound("test-sound-damage-bonus"),
        pause = asset.sound("test-sound-pause"),
        gauge_full = asset.sound("test-sound-gauge-full"),
        low_hp = asset.sound("test-sound-low-hp"),
        cut_in = asset.sound("test-sound-cut-in"),
        trap_mark = asset.sound("test-sound-trap-mark"),
        telop = asset.sound("test-sound-telop"),
        buster_charge = asset.sound("test-sound-buster-charge"),
        buster_charged = asset.sound("test-sound-buster-charged"),
        freeze = asset.sound("test-sound-freeze"),
        bubble = asset.sound("test-sound-bubble"),
        bubble_pop = asset.sound("test-sound-bubble-pop"),
        confusion = asset.sound("test-sound-confusion"),
        invisible = asset.sound("test-sound-invisible"),
        appear = asset.sound("test-sound-appear"),
        arrive = asset.sound("test-sound-arrive"),
        fade = asset.sound("test-sound-fade"),
        obstacle_lift = asset.sound("test-sound-obstacle-lift"),
        obstacle_throw = asset.sound("test-sound-obstacle-throw"),
        burst = asset.sound("test-sound-beast-over-burst"),
        refused = asset.sound("test-sound-refused"),
        custom_open = asset.sound("test-sound-custom-open"),
        custom_cursor = asset.sound("test-sound-custom-cursor"),
        custom_hide = asset.sound("test-sound-custom-hide"),
        custom_pick = asset.sound("test-sound-custom-pick"),
        custom_ok = asset.sound("test-sound-custom-ok"),
        custom_back = asset.sound("test-sound-custom-back"),
        custom_run_message = asset.sound("test-sound-custom-run-message"),
        custom_description = asset.sound("test-sound-custom-description"),
        custom_description_close = asset.sound("test-sound-custom-description-close"),
        custom_cancel = asset.sound("test-sound-custom-cancel"),
        custom_redeal = asset.sound("test-sound-custom-redeal"),
        custom_redeal_shuffle = asset.sound("test-sound-custom-redeal-shuffle"),
        custom_scrap = asset.sound("test-sound-custom-scrap"),
        custom_scrap_done = asset.sound("test-sound-custom-scrap-done"),
        program_advance_part = asset.sound("test-sound-program-advance-part"),
        program_advance = asset.sound("test-sound-program-advance"),
        encounter = asset.sound("test-sound-encounter"),
    },
    music = {
        link_battle = asset.sound("test-music-link-battle"),
        winner_special = asset.sound("test-music-winner-special"),
        winner = asset.sound("test-music-winner"),
        loser = asset.sound("test-music-loser"),
    },
    sprites = {
        charge_glow = sprites["test-sprite-charge-glow"],
        charge_glow_a = sprites["test-sprite-charge-glow-a"],
        full_synchro_aura = sprites["test-sprite-full-synchro-aura"],
        confusion = sprites["test-sprite-confusion"],
        blindness = sprites["test-sprite-blindness"],
        immobilized = sprites["test-sprite-immobilized"],
        ice = sprites["test-sprite-ice"],
        bubble = sprites["test-sprite-bubble"],
        hit_marker = sprites["test-sprite-hit-marker"],
        target_marker = sprites["test-sprite-lockon-marker"],
    },
    banners = {
        round_start = asset.banner("test-banner-round-start"),
        turn_start = asset.banner("test-banner-turn-start"),
        final_turn = asset.banner("test-banner-final-turn"),
        draw = asset.banner("test-banner-draw"),
        win = asset.banner("test-banner-win"),
        win_judged = asset.banner("test-banner-win-judged"),
        lose_judged = asset.banner("test-banner-lose-judged"),
        judge = asset.banner("test-banner-judge"),
        telop = asset.banner("test-banner-telop"),
        telop_remote = asset.banner("test-banner-telop-remote"),
        program_advance = asset.banner("test-banner-program-advance"),
        program_advance_empty = asset.banner("test-banner-program-advance-empty"),
    },
}
