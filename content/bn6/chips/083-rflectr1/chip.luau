--!strict
-- Rflectr1/2/3: action 0x2B (`sub_80ED13E`). See chips.md §3.3.
--
-- The navi holds up a shield (attack object #0x2B, its look from the
-- chip's second parameter) and guards for the chip's first parameter in
-- ticks. The first hit the guard blocks from the front sends a wave back
-- along the row (attack object #0x2F) with the chip's damage; a hit that
-- breaks guards takes the shield down (the guard with it). The Reflector
-- chips are subtypes 0 to 2; subtype 3 only guards, 4 fires the buster's
-- projectile back instead, and the others reflect nothing.

local reflected_shot = require("../../objects/reflected-shot/reflected_shot")
local reflector_shield = require("../../objects/reflector-shield/reflector_shield")

export type State = {
    -- Ticks guarded.
    timer: number,
}

local reflector = {
    state = { timer = "u16" },
}

-- The shield sits at the navi's sprite attach point 6.
local SHIELD_POINT = 6
-- A hit that breaks guards (the collision's hit flag 2).
local BREAKS_GUARD = 0x2
-- The wave's height, and its sound.
local WAVE_Z = 0x10_0000
local WAVE_SOUND = 0xC5
-- Subtype 4's shot: the buster's projectile (attack object #0,
-- `sub_80C4FFE`) at this height, of this kind (its first parameter).
local SHOT_Z = 0x14_0000
local SHOT_KIND = 6
-- (The register garbage the spawners get as the element: the literal 2
-- the flag test loaded.)
local LEFTOVER_ELEMENT = 2

-- The Beast forms whose head overlay mirrors the navi's animation.
local function beast_head_follows(form: number): boolean
    return form == 0xC or (form >= 0x12 and form <= 0x16) or form == 0x18
end

-- `sub_80101D4`: the Beast head (the navi's second related object) takes
-- animation `anim` too. (Without one, the game's stores go through a null
-- pointer and change nothing.)
local function set_head_animation(me: Object, anim: number)
    if beast_head_follows(battle.navi(me.alliance).form) then
        local head = me.related2
        if head then
            head.anim = anim
            head.anim_loaded = 0xFF
        end
    end
end

-- The shield goes, and the guard with it.
local function drop_shield(me: Object)
    me.related1 = nil
    me.overlay = nil
    me:set_status("guard", false)
end

-- The counterattack, one panel ahead (`loc_80ED1DC`).
local function reflect(me: Object)
    local x, y = me.panel_x + me.facing, me.panel_y
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    local variant = me.variant
    if variant <= 2 then
        reflected_shot.spawn(me, x, y, WAVE_Z, damage)
        battle.play_sound(WAVE_SOUND)
    elseif variant == 4 then
        -- sub_80C4FFE: attack object #0 (the buster's projectile) on (x, y).
        local o = battle.spawn("attack", 0, Vec3.new(y, LEFTOVER_ELEMENT, SHOT_Z), { SHOT_KIND, 0, 0, 0 })
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = LEFTOVER_ELEMENT
            o.damage = damage
            o.stamina = bit32.rshift(damage, 16)
            o.alliance, o.flip = me.alliance, me.flip
            o.related1 = me
        end
    end
end

-- `sub_80ED154`: raise the shield; then, each tick, reflect the first
-- blocked hit, until the time is up.
local function guard(me: Object, s: State)
    if me.step_init == 0 then
        local x, z = me:attach_point(SHIELD_POINT)
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        reflector_shield.spawn(me, Vec3.px(x, 0, z), params)
        me:set_status("guard", true)
        me:set_status("using_action", true)
        me:set_animation(0)
        set_head_animation(me, 0)
        me.marker = 0
        me.step_init = 4
        s.timer = 0
        return
    end
    -- Subtype 3 guards without looking at what it blocked.
    local blocked = bit32.btest(me.collision.guard_dirs, bit32.lshift(1, me.flip))
    if me.variant ~= 3 and blocked then
        if bit32.btest(me.collision.hit_flags, BREAKS_GUARD) then
            drop_shield(me)
        elseif me.marker == 0 then
            me.marker = 1
            -- (A subtype-3 case here would add 0x580 to the side's custom
            -- gauge, `sub_802E032`; subtype 3 never gets here.)
            reflect(me)
        end
    end
    local t = s.timer + 1
    s.timer = t
    if t > me:attack_param(1) then
        drop_shield(me)
        me:exit_attack()
    end
end

-- `sub_80ED13E`: one phase.
function reflector.update(me: Object, s: State)
    local step = me.step
    if step ~= 0 then
        error(string.format("Reflector phase %#x reads past its phase table", step))
    end
    guard(me, s)
end

return reflector
