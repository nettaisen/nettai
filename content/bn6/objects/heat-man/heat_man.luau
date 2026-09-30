--!strict
-- HeatMan, the navi of the HeatMan chips (actor object #0x07,
-- `sub_80B9078`), brought by their navi chip controller. He appears (with
-- his parts: a body overlay, AI index 1) and breathes fire in a widening
-- cone ahead of him: a flame on the panel ahead, three on the next column,
-- five on the one after (objects/heat-flame), on each panel that can hold
-- one. Then he leaves, and the controller moves on. See
-- docs/engine/chips.md §3.6.15.
--
-- The chips' script (navi chip subtype 2): `navi_chip` is his spawner,
-- `off_802CD5C[2]`.

local heat_flame = require("../heat-flame/heat_flame")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local heat_man = {
    state = { controller = "object" },
}

local SPRITE = "08-01"

-- His actor record (the parts it adds: his overlay, `sub_8010F6A`).
local ACTOR_TYPE: ActorType = "player"
local AI_INDEX = 1

-- Panel flags a flame needs (as one word), and his panel to stay.
local CAN_STAND = 0x10010

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080B921B

-- His flames' Param1 and burning time.
local FLAME_PARAM1, FLAME_TICKS = 4, 0x1E

-- `sub_80B921A`: HeatMan for `user`, reporting to `controller`.
function heat_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("heat-man", Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS), spec.params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o.damage = spec.damage
    o.stamina = bit32.rshift(spec.damage, 16)
    local s = o.state :: State
    s.controller = controller
    return o
end

-- `sub_80B909C`: on his panel, on the ground (Z's whole part cleared),
-- with his parts.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:add_navi_parts(ACTOR_TYPE, AI_INDEX, 1)
    me:set_lifecycle("update")
end

-- Enter the phase (phase 4; a word store of the timers when `word`), or
-- count down; true once the timer is 0 or below (or, `past_zero`, below
-- 0).
local function countdown(me: Object, ticks: number, word: boolean, past_zero: boolean): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer = ticks
        if word then
            me.timer2 = 0
        end
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return if past_zero then left < 0 else left <= 0
end

-- `sub_80B910C`: he appears; 5 ticks; on to standing, or straight to
-- leaving if his panel can't hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(0x94)
        me.visible = true
    end
    if not countdown(me, 5, true, false) then
        return
    end
    local stands = bit32.band(field.flags(me.panel_x, me.panel_y), CAN_STAND) == CAN_STAND
    me:set_action(if stands then 4 else 0x10)
end

-- `sub_80B9160`: 30 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if countdown(me, 0x1E, true, false) then
        me:set_action(8)
    end
end

-- `sub_80B9188`: 10 ticks drawing breath.
local function inhale(me: Object)
    if me.phase == 0 then
        me.anim = 0xF
    end
    if countdown(me, 0xA, false, false) then
        me:set_action(0xC)
    end
end

-- `sub_80B9288`: a flame on (x, y) if the panel can hold one.
local function flame(me: Object, x: number, y: number)
    if bit32.band(field.flags(x, y), CAN_STAND) ~= CAN_STAND then
        return
    end
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    -- r3 (0 << 16) becomes its Z.
    heat_flame.spawn(me, x, y, me.element, { FLAME_PARAM1, FLAME_TICKS, 0 }, damage, 0)
end

-- `sub_80B9240`: the cone: one, three, then five panels wide.
local function breathe(me: Object)
    local x, y, f = me.panel_x, me.panel_y, me.facing
    flame(me, x + f, y)
    for i = 0, 2 do
        flame(me, x + 2 * f, y - 1 + i)
    end
    for i = 0, 4 do
        flame(me, x + 3 * f, y - 2 + i)
    end
end

-- `sub_80B91B0`: the fire, 61 ticks (to -1); at 52 left he closes his
-- mouth.
local function fire(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x15
        breathe(me)
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x34 then
        me.anim = 0x10
        return
    end
    if left < 0 then
        me:set_action(0x10)
    end
end

-- `sub_80B91E8`: 5 ticks (to -1), then he goes (his parts with him) and
-- the controller moves on.
local function leave(me: Object)
    if me.phase == 0 then
        me.anim = 4
    end
    if not countdown(me, 4, true, true) then
        return
    end
    me:remove_navi_parts(ACTOR_TYPE, AI_INDEX)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = appear, [4] = stand, [8] = inhale, [0xC] = fire, [0x10] = leave }

-- `sub_80B9078`.
function heat_man.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("HeatMan action %#x reads past his table (off_80B90F8)", me.action))
        end
        action(me)
    else
        me:free()
    end
    me:update_sprite_while_dimmed()
end

return heat_man
