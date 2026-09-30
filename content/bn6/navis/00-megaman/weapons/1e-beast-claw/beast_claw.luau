--!strict
-- The Beast forms' claw: weapon routine 0x1E (`sub_8011E1C`), what a
-- charged Null-family chip becomes in a Beast form (the alternative
-- A-charge), and its action 0x52 (`sub_80EF534`). The chip-use framework
-- runs it inside the Beast Out rush, so it strikes from the panel in front
-- of its target: two slashes a panel ahead, each a one-tick hit. See
-- docs/engine/chips.md §4.6.

local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10).
    timer: number,
    -- Slashes still to come, this one included (AIAttackVars+0x12), which
    -- the setup counts from.
    slashes: number,
}

local beast_claw = {
    state = { timer = "u16", slashes = "u16" },
}

local ACTION = 0x52

-- The phases, on the attack's step.
local SLASH, RECOVER = 0x0, 0x4

-- The navi's (and its Beast head's) claw animation.
local CLAW_ANIM = 0x0C

-- The hit's counter byte.
local HIT_PARAM = 0x9E

-- Per slash, first and second (by 2 - slashes): the slash effect
-- (`byte_80EF606`), the hit's region shape (`byte_80EF5FC`'s records) and
-- hit modifier (`byte_80EF604`), and the sound.
local LOOKS = { [0] = 0x3A, [1] = 0x39 }
local REGIONS = { [0] = 0x02, [1] = 0x04 }
local HIT_MODS = { [0] = 1, [1] = 3 }
local FIRST_SOUND, SECOND_SOUND = 0x1C5, 0x1C6

-- The hit: no spark, against bodies (5), as an attack (4).
local HIT_EFFECT, TARGET, SELF_TYPE = 0xFF, 0x05, 0x04

-- `sub_8011E1C`: 50 damage and 10 more per point of buster damage (up to
-- 5), counter byte 0x9E, two slashes.
function beast_claw.setup(navi: Object): number
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = HIT_PARAM
    navi.attack_damage = weapon.charge_damage(navi, 0x32, 0xA)
    navi.attack_element = 0
    local s = navi:action_state(ACTION) :: State
    s.slashes = 2
    return ACTION
end

-- `sub_80EF550`: the claw raised (the Beast head's too) for 3 ticks, then
-- the slash effect and the hit a panel ahead.
local function slash(me: Object, s: State)
    if me.step_init == 0 then
        me:set_status("using_action", true)
        me.step_init = 1
        s.timer = 3
        me:set_animation(CLAW_ANIM)
        local head = me.related2
        if head then
            -- A halfword store: its loaded animation becomes 0, so it
            -- loads the claw next update.
            head.anim = CLAW_ANIM
            head.anim_loaded = 0
        end
    end
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    s.timer = 0xC
    me:open_counter_window()
    me.step = RECOVER
    me.step_init = 0
    -- (The game reads the count's low byte.)
    local count = bit32.band(s.slashes, 0xFF)
    battle.play_sound(if count == 2 then FIRST_SOUND else SECOND_SOUND)
    local k = 2 - count
    local look, region, hit_mod = LOOKS[k], REGIONS[k], HIT_MODS[k]
    if look == nil or region == nil or hit_mod == nil then
        error(string.format("beast claw slash %d reads past sub_80EF550's tables", k))
    end
    local x, y = me.panel_x + me.facing, me.panel_y
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0x10_0000), look, me.alliance)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        z = 0,
        region = region,
        hit_effect = HIT_EFFECT,
        target = TARGET,
        self_type = SELF_TYPE,
        damage = me.attack_damage,
        stamina = me.hit_param,
        hit_mod = hit_mod,
    })
end

-- `sub_80EF608`: 12 ticks, then the next slash, or idle after the last.
local function recover(me: Object, s: State)
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    local slashes = s.slashes - 1
    s.slashes = slashes
    if slashes ~= 0 then
        me.step = SLASH
        me.step_init = 0
        return
    end
    me:set_status("using_action", false)
    me:exit_attack()
end

-- `sub_80EF534`.
function beast_claw.update(me: Object, s: State)
    local step = me.step
    if step == SLASH then
        slash(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("beast claw phase %#x reads past sub_80EF534's phase table", step))
    end
end

return beast_claw
