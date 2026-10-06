--!strict
-- HeatMan, the navi of the HeatMan chips (the original's actor object
-- #0x07, `sub_80B9078`), brought by their navi chip controller. He appears
-- (with his parts: a body overlay, AI index 1) and breathes fire in a
-- widening cone ahead of him: a flame on the panel ahead, three on the next
-- column, five on the one after (./flame), on each panel that can hold
-- one. Then he leaves, and the controller moves on. See
-- docs/engine/chips.md §3.6.15.

local heatman_navi = require("../../navis/heatman").heatman
local collision = require("../../rules/collision")
local flame = require("./flame")
local navi_chips = require("../../lib/navi_chips/navi")

local heatman = {}

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
}

local SPRITE = asset.sprite("heatman")
local SOUND_APPEAR = asset.sound("appear")

-- What he wears (his overlay, `sub_8010F6A`): the link navi's own parts.
local IDENTITY = heatman_navi.identity

-- Panel flags a flame needs (as one word).
local CAN_HOLD = 0x10010

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080B921B

-- His flames act while dimmed and last 30 ticks, as attacks.
local FLAME: HeatFlame = { while_dimmed = true, ticks = 0x1E, is = collision.attack }

-- His actions.
local APPEAR, STAND, INHALE, FIRE, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_80B909C`: on his panel, on the ground (Z's whole part cleared),
-- with his parts.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground")
    me:add_parts(IDENTITY, 1)
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
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if not countdown(me, 5, true, false) then
        return
    end
    me:set_action(if navi_chips.has_footing(me) then STAND else LEAVE)
end

-- `sub_80B9160`: 30 ticks standing.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if countdown(me, 0x1E, true, false) then
        me:set_action(INHALE)
    end
end

-- `sub_80B9188`: 10 ticks drawing breath.
local function inhale(me: Object)
    if me.phase == 0 then
        me.anim = 0xF
    end
    if countdown(me, 0xA, false, false) then
        me:set_action(FIRE)
    end
end

-- `sub_80B9288`: a flame on (x, y) if the panel can hold one.
local function set_alight(me: Object, x: number, y: number)
    if bit32.band(field.flags(x, y), CAN_HOLD) ~= CAN_HOLD then
        return
    end
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    -- The registers (the row, the element and r3, 0 << 16) are its
    -- position until its init.
    flame.spawn(me, x, y, me.element, FLAME, damage, Vec3.new(y, me.element, 0))
end

-- `sub_80B9240`: the cone: one, three, then five panels wide.
local function breathe(me: Object)
    local x, y, f = me.panel_x, me.panel_y, me.facing
    set_alight(me, x + f, y)
    for i = 0, 2 do
        set_alight(me, x + 2 * f, y - 1 + i)
    end
    for i = 0, 4 do
        set_alight(me, x + 3 * f, y - 2 + i)
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
        me:set_action(LEAVE)
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
    me:remove_parts(IDENTITY)
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = { [APPEAR] = appear, [STAND] = stand, [INHALE] = inhale, [FIRE] = fire, [LEAVE] = leave }

heatman.kind = new.kind {
    id = "heatman/navi",
    pool = "actor",
    state = { controller = "object" },
    -- `sub_80B9078`.
    update = function(me: Object)
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
    end,
}

-- `sub_80B921A`: HeatMan for `user`, reporting to `controller` (the
-- HeatMan chips' navi hook, `off_802CD5C[2]`). His position is the
-- spawner's registers until his init places him.
function heatman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(heatman.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.controller = controller
    return o
end

return heatman
