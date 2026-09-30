--!strict
-- A thrown seed (attack object #0x4F, `sub_80CE24C`): PoisSeed, IceSeed
-- and GrasSeed (action 0x12). It flies three panels ahead in the bomb's
-- arc and lands: on a solid panel it hits it for a tick, then turns the
-- panels around it into its kind of panel, only its own if the hit landed
-- on something; off the field or on a hole it just ends (a splash in the
-- hole). What it is, its first parameter, and its palette, the second.

local panels = require("../../lib/panels")
local region = require("../../lib/region")

local seed = {}

-- `sub_8011504`: sprite 0x0C-0x02, animation 9, with a ground shadow;
-- collision self type 4, target 5, hit modifier 1.
local SPRITE, ANIM = "0c-02", 9
local SELF_TYPE, TARGET, HIT_MOD = 0x04, 0x05, 1
-- Flight: as the bomb's (`byte_80CE3AC`).
local FLIGHT_TICKS = 0x28
local SPEED, RISE, GRAVITY = 0x2E666, 0x20666, -0x2800
-- Its panel when the hit connected, the 3x3 around it when not.
local REGION_HIT, REGION_MISS = 0x01, 0x0F
local EFFECT_SPLASH, EFFECT_GONE = 1, 0x14

-- By kind: its element (`dword_80CE2B0`); what it grows (`byte_80CE41E`),
-- the effect on each panel (`byte_80CE421`) and its sound
-- (`byte_80CE418`). Kind 3 is fire, and reads past the other tables.
local ELEMENTS = { [0] = 0, [1] = 2, [2] = 4, [3] = 1 }
type Kind = { panel: PanelType, effect: number, sound: number }
local KINDS: { [number]: Kind } = {
    [0] = { panel = "poison", effect = 0x3B, sound = 0x90 },
    [1] = { panel = "ice", effect = 0x3B, sound = 0x118 },
    [2] = { panel = "grass", effect = 0x3C, sound = 0x11B },
}

-- `sub_80CE44E`: a seed thrown by `user` from `pos`, with the chip's
-- parameters and damage word.
function seed.throw(user: Object, pos: Vec3, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("seed", pos, params)
    if o then
        o.alliance, o.flip = user.alliance, user.flip
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.related1 = user
    end
    return o
end

-- Done: hidden, off its panels, and freed at the next update.
local function finish(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80CE308`: fly; on the last tick, land.
local function fly(me: Object)
    if me.phase_init == 0 then
        me.vel = Vec3.new(SPEED * me.facing, GRAVITY, RISE)
        me.timer = FLIGHT_TICKS
        me.phase_init = 4
    end
    local p, v = me.pos, me.vel
    local vz = int.i32(v.z + v.y)
    me.pos = Vec3.new(p.x + v.x, p.y, p.z + vz)
    me.vel = Vec3.new(v.x, v.y, vz)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    me.visible = false
    me:set_panel_from_coordinates()
    local x, y = me.panel_x, me.panel_y
    me.collision.panel_x, me.collision.panel_y = x, y
    if not field.valid(x, y) then
        me.collision.region = 0
        me:set_lifecycle("destroy")
        me.visible = false
        return
    end
    if not field.solid(x, y) then
        battle.effect(me.pos, EFFECT_SPLASH)
        me.collision.region = 0
        me:set_lifecycle("destroy")
        me.visible = false
        return
    end
    me.collision.region = 1
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- `sub_80CE3C4`: the tick after landing: grow the panels, and end.
local function burst(me: Object)
    me.collision.region = 0
    local r = if me.collision.hit_flags ~= 0 then REGION_HIT else REGION_MISS
    local kind = me:param(1)
    local k = KINDS[kind]
    if not k then
        error(string.format("seed kind %d reads past byte_80CE41E", kind))
    end
    local x, y = me.panel_x, me.panel_y
    -- (The game passes the side and flip halfword as the side, unmasked,
    -- so a flipped seed's dx lands far off the field.)
    local dir = panels.side_direction(me.alliance + me.flip * 0x100)
    region.effects_on_panels(x, y, k.effect, r, dir, 0)
    region.set_types(x, y, k.panel, r)
    battle.play_sound(k.sound)
    me:set_lifecycle("destroy")
end

-- `sub_80CE2B4`: resolve last tick's hits; end with the battle; stand
-- still while dimmed.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    if battle.over() then
        battle.effect(me.pos, EFFECT_GONE)
        finish(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a == 0 then
            fly(me)
        elseif a == 4 then
            burst(me)
        else
            error(string.format("seed action %#x reads past its table", a))
        end
    end
    me.collision:present()
end

-- `sub_80CE270`.
local function init(me: Object)
    local kind = me:param(1)
    local element = ELEMENTS[kind]
    if element == nil then
        error(string.format("seed kind %d reads past dword_80CE2B0", kind))
    end
    me.element = element
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = ANIM
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    -- `sub_801A146`: the hit spark is the element's.
    me.collision.hit_effect = bit32.band(me.element, 0xF)
    me.collision:present()
    sprite.palette = me:param(2)
    me.visible = true
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80CE24C`.
function seed.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
        if not me.active then
            return
        end
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
        return
    end
    me:update_sprite()
end

return seed
