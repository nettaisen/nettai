--!strict
-- DustMan (the original's actor object #0x18, `sub_80BBF0C`), the navi of
-- the DustMan chips, brought by the navi chip controller. He appears, takes
-- every field object he can, rolls a ball of junk forward (DustCross's
-- junk ball, objects/dust-ball, running on while dimmed), then throws what
-- he took back at the other side one by one, 20 ticks apart (./junk). See
-- docs/engine/chips.md §3.6.26.

local dust_ball = require("../../objects/dust-ball/dust_ball")
local junk = require("./junk")

local dustman = {}

export type State = {
    -- What he took: their junk looks (his ExtraVars), how many (Param1)
    -- and how many he has thrown back (Param2).
    taken: { number },
    count: number,
    thrown: number,
    -- The controller waiting for him.
    controller: Object?,
}

local SPRITE = asset.sprite("dustman")
local SOUND_APPEAR = asset.sound("appear")
local SOUND_TAKE = asset.sound("dustbrk")
local SOUND_THROW = asset.sound("junk-shot")

-- The controller calls his spawner through r3, which his position takes:
-- its address with the Thumb bit, whose fraction his Z keeps.
local SPAWNER_ADDRESS = 0x080BC0DB

-- Panel flags he needs to stay (as one word).
local CAN_STAND = 0x10010

-- Ticks between throws.
local THROW_TICKS = 0x14

-- His actions.
local APPEAR, TAKE, WIND_UP, THROW, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_80BBF30`: on his panel, on the ground (Z's whole part cleared).
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

-- Enter the phase (phase 4; a word store of the timers), or count down;
-- the timer's new value.
local function countdown(me: Object, ticks: number): number?
    if me.phase == 0 then
        me.phase = 4
        me.timer = ticks
        me.timer2 = 0
        return nil
    end
    local left = me.timer - 1
    me.timer = left
    return left
end

-- `sub_80BBF94`: he appears; 10 ticks; on, or away if his panel can't
-- hold him.
local function appear(me: Object)
    if me.phase == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    local left = countdown(me, 0xA)
    if left == nil or left > 0 then
        return
    end
    local stands = bit32.band(field.flags(me.panel_x, me.panel_y), CAN_STAND) == CAN_STAND
    me:set_action(if stands then TAKE else LEAVE)
end

-- `sub_80BC100`: every field object he can take, in the registry's slot
-- order: its junk look kept, and the chip's removal asked of it. How many.
local function take(me: Object): number
    local s = me.state :: State
    local n = 0
    for _, o in field.all_objects() do
        local look = obstacle.junk_look(o)
        if look then
            n += 1
            s.taken[n] = look
            obstacle.remove(o)
        end
    end
    return n
end

-- `sub_80BBFE8`: 20 ticks; at 10 he takes the field's objects.
local function suck(me: Object)
    local left = countdown(me, 0x14)
    if left == nil then
        return
    end
    if left == 0xA then
        local s = me.state :: State
        s.count = take(me)
        if s.count ~= 0 then
            battle.play_sound(SOUND_TAKE)
        end
        return
    end
    if left <= 0 then
        me:set_action(WIND_UP)
    end
end

-- `sub_80BC024`: 34 ticks.
local function wind_up(me: Object)
    if me.phase == 0 then
        me.anim = 0xF
    end
    local left = countdown(me, 0x22)
    if left == nil then
        return
    end
    if left == 9 then
        me.anim = 0x11
        return
    end
    if left <= 0 then
        me:set_action(THROW)
    end
end

-- `sub_80BC058`: the junk ball rolls (`sub_80BC13E`), then a piece of junk
-- every 20 ticks, until 20 more ticks than pieces have passed.
local function throw(me: Object)
    local s = me.state :: State
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    if me.phase == 0 then
        me.phase = 4
        dust_ball.spawn(me, me.panel_x, me.panel_y, me.element, damage, true)
        battle.play_sound(SOUND_THROW)
        s.thrown = 0
        me.timer = (s.count + 1) * THROW_TICKS
        me.timer2 = THROW_TICKS
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(LEAVE)
        return
    end
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 > 0 then
        return
    end
    -- sub_80BC160: the next piece (its look the next one he took), its Z
    -- the r3 the object loop left him.
    junk.throw(me, me.panel_x, me.panel_y, me.element, damage, s.taken[s.thrown + 1] or 0, battle.loop_register())
    battle.play_sound(SOUND_THROW)
    s.thrown += 1
    me.timer2 = THROW_TICKS
end

-- `sub_80BC0B0`: 4 ticks (to -1), then he goes and the controller moves on.
local function leave(me: Object)
    if me.phase == 0 then
        me.anim = 4
    end
    local left = countdown(me, 4)
    if left == nil or left >= 0 then
        return
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:set_lifecycle("destroy")
end

local ACTIONS = { [APPEAR] = appear, [TAKE] = suck, [WIND_UP] = wind_up, [THROW] = throw, [LEAVE] = leave }

dustman.kind = define.kind {
    id = "dustman/navi",
    pool = "actor",
    state = { taken = "u16[8]", count = "u8", thrown = "u8", controller = "object" },
    -- `sub_80BBF0C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("DustMan action %#x reads past his table (off_80BBF80)", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80BC0DA`: DustMan for `user`, reporting to `controller` (the
-- DustMan chips' navi hook, `off_802CD5C[11]`).
function dustman.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = battle.spawn(dustman.kind, Vec3.new(spec.panel_y, spec.element, SPAWNER_ADDRESS))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o:set_damage_word(spec.damage)
    local s = o.state :: State
    s.controller = controller
    return o
end

return dustman
