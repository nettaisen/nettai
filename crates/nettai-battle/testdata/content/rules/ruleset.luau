--!strict
-- The engine's test content's own effects, hit sparks, region, collision
-- types, sounds, music and banners for the ruleset's roles
-- (rules/roles.luau): made-up looks (one sprite for every effect, one for
-- every spark), collision types by what they are, and a made-up asset for
-- every sound, music and banner.

local function effect(): Effect
    return define.effect { sprite = asset.sprite("test:test-burst"), anim = 0, palette = 0 }
end

local function spark(): Spark
    return define.spark { sprite = asset.sprite("test:test-spark"), anim = 0, palette = 0 }
end

-- A collision type's flag words: an attack, an object and a body of each
-- side, another body, a neutral object, a player's body, a floating body;
-- what resolves while the battle is dimmed, what reaches floating bodies.
local ATTACK = { [0] = 0x80000000, [1] = 0x40000000 }
local OBJECT = { [0] = 0x20000000, [1] = 0x10000000 }
local BODY = { [0] = 0x08000000, [1] = 0x04000000 }
local OTHER_BODY = { [0] = 0x02000000, [1] = 0x01000000 }
local NEUTRAL = 0x00800000
local PLAYER = { [0] = 0x00400000, [1] = 0x00200000 }
local FLOATING = 0x00100000
local WHILE_DIMMED = 0x00010000
local REACHES_FLOATING = 0x0080

-- A type whose flags `flags` gives for each side. `row` is the row the
-- type has in BN6's table: a bug code's high byte takes its offset.
local function collision(id: string, row: number, flags: (side: number) -> number): CollisionType
    return define.collision { id = "test:test/" .. id, side0 = flags(0), side1 = flags(1), row_offset = row * 8 }
end

local function body(side: number): number
    return bit32.bor(BODY[side], PLAYER[side], WHILE_DIMMED, REACHES_FLOATING)
end

local function attack(side: number): number
    return bit32.bor(ATTACK[side], REACHES_FLOATING)
end

-- The other side's objects and bodies, and neutral objects.
local function others(side: number): number
    local other = 1 - side
    return bit32.bor(OBJECT[other], BODY[other], OTHER_BODY[other], NEUTRAL)
end

return {
    effects = {
        deletion = effect(),
        recovery = effect(),
        cut_in_flash = effect(),
        trap_mark = effect(),
        encased = effect(),
        beast_over_burst = effect(),
        panel_splash = effect(),
    },
    sparks = {
        plain = spark(),
        guard = spark(),
        eruption = spark(),
        thrown_obstacle = spark(),
        uninstall = spark(),
        panel_burn = spark(),
    },
    regions = {
        anchor = define.region { panels = { { 0, 0 } } },
    },
    collision = {
        navi = collision("navi", 0x01, body),
        floating_navi = collision("floating-navi", 0x10, function(side)
            return bit32.bor(body(side), FLOATING)
        end),
        navi_target = collision("navi-target", 0x02, function(side)
            return bit32.bor(ATTACK[1 - side], others(side))
        end),
        eruption = collision("eruption", 0x48, attack),
        eruption_target = collision("eruption-target", 0x2A, others),
        thrown_obstacle = collision("thrown-obstacle", 0x06, attack),
        thrown_obstacle_target = collision("thrown-obstacle-target", 0x05, others),
    },
    -- Made-up sounds, music, sprites and banners, one for each role (named
    -- `test-<kind>-<role>`; the test content gives each the id the tests
    -- look for).
    sounds = {
        panel_crack = asset.sound("test:test-sound-panel-crack"),
        panel_poison = asset.sound("test:test-sound-panel-poison"),
        own_hit = asset.sound("test:test-sound-own-hit"),
        hit = asset.sound("test:test-sound-hit"),
        damage = asset.sound("test:test-sound-damage"),
        guard = asset.sound("test:test-sound-guard"),
        counter_hit = asset.sound("test:test-sound-counter-hit"),
        deleted = asset.sound("test:test-sound-deleted"),
        recovery = asset.sound("test:test-sound-recovery"),
        damage_bonus = asset.sound("test:test-sound-damage-bonus"),
        pause = asset.sound("test:test-sound-pause"),
        gauge_full = asset.sound("test:test-sound-gauge-full"),
        low_hp = asset.sound("test:test-sound-low-hp"),
        cut_in = asset.sound("test:test-sound-cut-in"),
        telop = asset.sound("test:test-sound-telop"),
        buster_charge = asset.sound("test:test-sound-buster-charge"),
        buster_charged = asset.sound("test:test-sound-buster-charged"),
        freeze = asset.sound("test:test-sound-freeze"),
        bubble = asset.sound("test:test-sound-bubble"),
        bubble_pop = asset.sound("test:test-sound-bubble-pop"),
        confusion = asset.sound("test:test-sound-confusion"),
        invisible = asset.sound("test:test-sound-invisible"),
        appear = asset.sound("test:test-sound-appear"),
        arrive = asset.sound("test:test-sound-arrive"),
        fade = asset.sound("test:test-sound-fade"),
        obstacle_lift = asset.sound("test:test-sound-obstacle-lift"),
        obstacle_throw = asset.sound("test:test-sound-obstacle-throw"),
        beast_over_burst = asset.sound("test:test-sound-beast-over-burst"),
        cross_special = asset.sound("test:test-sound-cross-special"),
        refused = asset.sound("test:test-sound-refused"),
        custom_open = asset.sound("test:test-sound-custom-open"),
        custom_cursor = asset.sound("test:test-sound-custom-cursor"),
        custom_hide = asset.sound("test:test-sound-custom-hide"),
        custom_pick = asset.sound("test:test-sound-custom-pick"),
        custom_ok = asset.sound("test:test-sound-custom-ok"),
        custom_back = asset.sound("test:test-sound-custom-back"),
        custom_cross_open = asset.sound("test:test-sound-custom-cross-open"),
        custom_cross_close = asset.sound("test:test-sound-custom-cross-close"),
        custom_cross_chosen = asset.sound("test:test-sound-custom-cross-chosen"),
        custom_run_message = asset.sound("test:test-sound-custom-run-message"),
        custom_description = asset.sound("test:test-sound-custom-description"),
        custom_description_close = asset.sound("test:test-sound-custom-description-close"),
        custom_beast_out_falzar = asset.sound("test:test-sound-custom-beast-out-falzar"),
        custom_beast_out_gregar = asset.sound("test:test-sound-custom-beast-out-gregar"),
        custom_beast_out_flash = asset.sound("test:test-sound-custom-beast-out-flash"),
        custom_cancel = asset.sound("test:test-sound-custom-cancel"),
        custom_redeal = asset.sound("test:test-sound-custom-redeal"),
        custom_redeal_shuffle = asset.sound("test:test-sound-custom-redeal-shuffle"),
        custom_scrap = asset.sound("test:test-sound-custom-scrap"),
        custom_scrap_done = asset.sound("test:test-sound-custom-scrap-done"),
        program_advance_part = asset.sound("test:test-sound-program-advance-part"),
        program_advance = asset.sound("test:test-sound-program-advance"),
    },
    music = {
        link_battle = asset.sound("test:test-music-link-battle"),
        winner_special = asset.sound("test:test-music-winner-special"),
        winner = asset.sound("test:test-music-winner"),
        loser = asset.sound("test:test-music-loser"),
    },
    sprites = {
        charge_glow = asset.sprite("test:test-sprite-charge-glow"),
        charge_glow_a = asset.sprite("test:test-sprite-charge-glow-a"),
        full_synchro_aura = asset.sprite("test:test-sprite-full-synchro-aura"),
        confusion = asset.sprite("test:test-sprite-confusion"),
        blindness = asset.sprite("test:test-sprite-blindness"),
        immobilized = asset.sprite("test:test-sprite-immobilized"),
        ice = asset.sprite("test:test-sprite-ice"),
        bubble = asset.sprite("test:test-sprite-bubble"),
        hit_marker = asset.sprite("test:test-sprite-hit-marker"),
        eruption = asset.sprite("test:test-sprite-eruption"),
        lockon_marker = asset.sprite("test:test-sprite-lockon-marker"),
    },
    banners = {
        round_start = asset.banner("test:test-banner-round-start"),
        turn_start = asset.banner("test:test-banner-turn-start"),
        final_turn = asset.banner("test:test-banner-final-turn"),
        draw = asset.banner("test:test-banner-draw"),
        judge = asset.banner("test:test-banner-judge"),
        telop = asset.banner("test:test-banner-telop"),
        telop_remote = asset.banner("test:test-banner-telop-remote"),
        program_advance = asset.banner("test:test-banner-program-advance"),
        program_advance_empty = asset.banner("test:test-banner-program-advance-empty"),
    },
}
