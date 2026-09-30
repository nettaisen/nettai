--!strict
-- SpoutMan, the navi of the SpoutMan chips (actor object #0x09,
-- `sub_80B94BC`), brought by their navi chip controller. He appears in his
-- water (his layer: the parts of his actor record, AI index 6), and
-- attacks by where he stands:
--
--   away from his back columns: he throws a water ball two columns ahead
--   (objects/spout-ball), which splashes there and one column beyond;
--   in them: the water under him spouts (objects/spout-pillar) and a
--   geyser erupts in the first column ahead holding an enemy navi, or the
--   field's edge, hitting that column and the trail to it three times
--   (objects/spout-geyser, its marks objects/spout-mark).
--
-- Then he leaves, and the controller moves on. See docs/engine/chips.md
-- §3.6.11.
--
-- The chips' script (navi chip subtype 7): `navi_chip` is his spawner,
-- `off_802CD5C[7]`.

local panels = require("../../lib/panels")
local spout_ball = require("../spout-ball/spout_ball")
local spout_pillar = require("../spout-pillar/spout_pillar")
local spout_geyser = require("../spout-geyser/spout_geyser")

export type State = {
    -- The controller waiting for him (its pointer sits in his collision
    -- data slot).
    controller: Object?,
    -- The water spouting under him (ExtraVars[0], which it clears when it
    -- goes).
    pillar: Object?,
}

local spout_man = {
    state = { controller = "object", pillar = "object" },
}

local SPRITE = "08-06"

-- His actor record (the parts it adds: his layer, `sub_8010F7A`).
local ACTOR_TYPE: ActorType = "player"
local AI_INDEX = 6

-- Panel flags he needs to stay (as one word).
local CAN_STAND = 0x10010

-- What his geyser looks for, by side (`off_80B97B8`: an enemy navi's
-- body).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }

-- His attacks' Param1, and how long his geyser lasts.
local PARAM1 = 4
local GEYSER_TICKS = 0x5A

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080B9751

-- The pillar's signals (its phase_init byte).
local SPOUT, DONE = 1, 2

-- `sub_80B9750`: SpoutMan for `user`, reporting to `controller`.
function spout_man.navi_chip(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn_kind("spout-man", Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS), spec.params)
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

-- `sub_80B94E0`: on his panel, on the ground (Z's whole part cleared).
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
    me:set_lifecycle("update")
end

-- Enter the phase (phase 4, a word store of the timers), or count down;
-- true once the timer is 0 or below.
local function countdown(me: Object, ticks: number): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer, me.timer2 = ticks, 0
        return false
    end
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- `sub_80B954C`: he appears with his layer; 6 ticks; on to standing, or
-- straight to leaving if his panel can't hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(0x94)
        me:add_navi_parts(ACTOR_TYPE, AI_INDEX, 1)
        me.visible = true
    end
    if not countdown(me, 6) then
        return
    end
    local stands = bit32.band(field.flags(me.panel_x, me.panel_y), CAN_STAND) == CAN_STAND
    me:set_action(if stands then 4 else 0x18)
end

-- `sub_80B95AC`: 30 ticks standing; then, away from his back columns, the
-- water ball; in them, the geyser.
local function stand(me: Object)
    if me.phase == 0 then
        me.anim = 0
    end
    if not countdown(me, 0x1E) then
        return
    end
    local x = me.panel_x
    local forward = if me.alliance == 0 then x > 2 else x < 5
    me:set_action(if forward then 8 else 0x10)
end

-- `sub_80B95F2`: 20 ticks winding up the throw.
local function wind_up(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x13
        me.timer = 0x14
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0xC)
    end
end

-- `sub_80B961A`: the throw: a water ball and a splash effect where he is;
-- 60 ticks.
local function throw(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x14
        spout_ball.spawn(me, me.panel_x, me.panel_y, me.element, { PARAM1 }, damage_word(me))
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, 0), 0x2A, bit32.bxor(me.alliance, me.flip))
        battle.play_sound(0x12D)
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(0x18)
    end
end

-- `sub_80B966C`: the water under him rises (its pillar); at 57 left he
-- raises his arms, at 27 the pillar spouts; then the geyser.
local function rise(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0xF
        spout_pillar.spawn(me, me.panel_x, me.panel_y, { PARAM1 })
        me.timer = 0x42
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x39 then
        me.anim = 0x10
        return
    end
    if left == 0x1B then
        local pillar = (me.state :: State).pillar
        if pillar then
            pillar.phase_init = SPOUT
        end
        return
    end
    if left <= 0 then
        me:set_action(0x14)
    end
end

-- `sub_80B9776`: the column his geyser aims at: the first one ahead with
-- an enemy navi in it, or the field's edge.
local function target_column(me: Object): number
    local i = 1
    while true do
        local x = me.panel_x + me.facing * i
        if #panels.in_column(x, ENEMY_NAVI[me.alliance], 0) ~= 0 or x <= 1 or x >= 6 then
            return x
        end
        i += 1
    end
end

-- `sub_80B96C2`: the geyser, 90 ticks; then his pillar is done.
local function geyser(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 0x15
        battle.play_sound(0x189)
        me.timer = GEYSER_TICKS
        local target = target_column(me)
        -- r3 (4, the geyser's Param1) becomes its Z.
        spout_geyser.spawn(me, me.panel_x, me.panel_y, me.element, { PARAM1, GEYSER_TICKS, target }, damage_word(me), 4)
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.anim = 0
    local pillar = (me.state :: State).pillar
    if pillar then
        pillar.phase_init = DONE
    end
    me:set_action(0x18)
end

-- `sub_80B971E`: 9 ticks (to -1), then he goes (his layer with him) and
-- the controller moves on.
local function leave(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 4
        me.timer, me.timer2 = 8, 0
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me:remove_navi_parts(ACTOR_TYPE, AI_INDEX)
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = {
    [0] = appear,
    [4] = stand,
    [8] = wind_up,
    [0xC] = throw,
    [0x10] = rise,
    [0x14] = geyser,
    [0x18] = leave,
}

-- `sub_80B94BC`.
function spout_man.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("SpoutMan action %#x reads past his table (off_80B9530)", me.action))
        end
        action(me)
    else
        me:free()
    end
    me:update_sprite_while_dimmed()
end

return spout_man
