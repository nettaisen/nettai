--!strict
-- The engine's test content's own effects, hit sparks, region, collision
-- types, sounds, music and banners for the rules' roles
-- (rules/roles.luau): made-up looks (one sprite for every effect, one for
-- every spark), collision types by what they are, and a made-up asset for
-- every sound, music and banner.

local sprites = require("../sprites")

local function effect(): Effect
    return new.effect { sprite = sprites.test_burst, anim = 0, palette = 0, steps = 9 }
end

local function spark(): Spark
    return new.spark { sprite = sprites.test_spark, anim = 0, palette = 0, steps = 4 }
end

-- A type of members `flags` (an own side's of the object's side, the
-- other side's mirrored).
local function collision(id: string, flags: { [CollisionMember]: boolean }): CollisionType
    return new.collision { id = "test/" .. id, flags = flags }
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
        navi = collision("navi", BODY),
        floating_navi = collision(
            "floating_navi",
            { own_body = true, own_navi = true, floating = true, hits_while_dimmed = true, reaches_float_shoe = true }
        ),
        navi_target = collision(
            "navi_target",
            { enemy_attack = true, enemy_object = true, enemy_body = true, enemy_other_body = true, neutral = true }
        ),
        thrown_obstacle = collision("thrown_obstacle", { own_attack = true, reaches_float_shoe = true }),
        thrown_obstacle_target = collision("thrown_obstacle_target", OTHERS),
    },
    -- Made-up sounds, music, sprites and banners, one for each role (named
    -- `test-<kind>-<role>`; the test content gives each the id the tests
    -- look for).
    sounds = {
        panel_crack = asset.sound("test_sound_panel_crack"),
        own_hit = asset.sound("test_sound_own_hit"),
        hit = asset.sound("test_sound_hit"),
        damage = asset.sound("test_sound_damage"),
        guard = asset.sound("test_sound_guard"),
        counter_hit = asset.sound("test_sound_counter_hit"),
        deleted = asset.sound("test_sound_deleted"),
        recovery = asset.sound("test_sound_recovery"),
        damage_bonus = asset.sound("test_sound_damage_bonus"),
        pause = asset.sound("test_sound_pause"),
        gauge_full = asset.sound("test_sound_gauge_full"),
        low_hp = asset.sound("test_sound_low_hp"),
        cut_in = asset.sound("test_sound_cut_in"),
        trap_mark = asset.sound("test_sound_trap_mark"),
        telop = asset.sound("test_sound_telop"),
        buster_charge = asset.sound("test_sound_buster_charge"),
        buster_charged = asset.sound("test_sound_buster_charged"),
        freeze = asset.sound("test_sound_freeze"),
        bubble = asset.sound("test_sound_bubble"),
        bubble_pop = asset.sound("test_sound_bubble_pop"),
        confusion = asset.sound("test_sound_confusion"),
        invisible = asset.sound("test_sound_invisible"),
        appear = asset.sound("test_sound_appear"),
        arrive = asset.sound("test_sound_arrive"),
        fade = asset.sound("test_sound_fade"),
        obstacle_lift = asset.sound("test_sound_obstacle_lift"),
        obstacle_throw = asset.sound("test_sound_obstacle_throw"),
        burst = asset.sound("test_sound_beast_over_burst"),
        refused = asset.sound("test_sound_refused"),
        custom_open = asset.sound("test_sound_custom_open"),
        custom_cursor = asset.sound("test_sound_custom_cursor"),
        custom_hide = asset.sound("test_sound_custom_hide"),
        custom_pick = asset.sound("test_sound_custom_pick"),
        custom_ok = asset.sound("test_sound_custom_ok"),
        custom_back = asset.sound("test_sound_custom_back"),
        custom_run_message = asset.sound("test_sound_custom_run_message"),
        custom_description = asset.sound("test_sound_custom_description"),
        custom_description_close = asset.sound("test_sound_custom_description_close"),
        custom_cancel = asset.sound("test_sound_custom_cancel"),
        custom_redeal = asset.sound("test_sound_custom_redeal"),
        custom_redeal_shuffle = asset.sound("test_sound_custom_redeal_shuffle"),
        custom_scrap = asset.sound("test_sound_custom_scrap"),
        custom_scrap_done = asset.sound("test_sound_custom_scrap_done"),
        program_advance_part = asset.sound("test_sound_program_advance_part"),
        program_advance = asset.sound("test_sound_program_advance"),
        encounter = asset.sound("test_sound_encounter"),
    },
    music = {
        link_battle = asset.sound("test_music_link_battle"),
        winner_special = asset.sound("test_music_winner_special"),
        winner = asset.sound("test_music_winner"),
        loser = asset.sound("test_music_loser"),
    },
    sprites = {
        charge_glow = sprites.test_sprite_charge_glow,
        charge_glow_a = sprites.test_sprite_charge_glow_a,
        full_synchro_aura = sprites.test_sprite_full_synchro_aura,
        confusion = sprites.test_sprite_confusion,
        blindness = sprites.test_sprite_blindness,
        immobilized = sprites.test_sprite_immobilized,
        ice = sprites.test_sprite_ice,
        bubble = sprites.test_sprite_bubble,
        hit_marker = sprites.test_sprite_hit_marker,
        target_marker = sprites.test_sprite_lockon_marker,
    },
    banners = {
        round_start = asset.banner("test_banner_round_start"),
        turn_start = asset.banner("test_banner_turn_start"),
        final_turn = asset.banner("test_banner_final_turn"),
        draw = asset.banner("test_banner_draw"),
        win = asset.banner("test_banner_win"),
        win_judged = asset.banner("test_banner_win_judged"),
        lose_judged = asset.banner("test_banner_lose_judged"),
        judge = asset.banner("test_banner_judge"),
        telop = asset.banner("test_banner_telop"),
        telop_remote = asset.banner("test_banner_telop_remote"),
        program_advance = asset.banner("test_banner_program_advance"),
        program_advance_empty = asset.banner("test_banner_program_advance_empty"),
    },
}
