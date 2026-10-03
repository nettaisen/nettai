--!strict
-- A segment of Sensor's laser (the original's attack object #0xA0,
-- `sub_80D9350`; docs/engine/dimming-chips.md §8.4): on its panel at its
-- aim's height, its hits paralyze. Once it has hit something its region
-- goes, and comes back once its panel is free of bodies and objects. It
-- registers in its turret's `segments` and clears its entry when the
-- turret ends it.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

local laser = {}

export type State = {
    -- Its turret, and its entry there.
    turret: Object?,
    slot: number,
    -- Its aim (Param1): 0 up, 1 straight, 2 down.
    aim: number,
    -- The battle was paused at its last update (ExtraVars+8).
    was_paused: boolean,
}

local SPRITE = asset.sprite("bn6:erase-beam")
-- By aim: the height in whole pixels (`byte_80D9410`) and the animation
-- (`dword_80D9408`).
local HEIGHTS: { [number]: number } = { [0] = 0x10, [1] = 0x16, [2] = 0x14 }
local ANIMS: { [number]: number } = { [0] = 1, [1] = 0, [2] = 2 }
-- Its hits' status (`byte_80D941C`'s row +0x64, its Param3, which its
-- spawner leaves 0): paralysis.
local STATUS = statuses.paralyze_90
-- The ticks before it shows (its Param2, which its spawner leaves 0: at
-- once).
local DELAY = 0
-- Hit effect 3: an elec attack's spark.
local SPARK = sparks.of_element(3)
-- Its panel must be free of bodies and objects to arm again.
local OCCUPIED = 0x0F88_0080

-- The actions (the object's action byte).
local WAIT, ARMED, SPENT, ENDED = 0, 4, 8, 0xC

-- `sub_80D937C`: on its panel at its aim's height, hidden, its collision
-- on with no region yet.
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local height, anim = HEIGHTS[s.aim], ANIMS[s.aim]
    if not height then
        error(string.format("Sensor's laser: aim %d reads past byte_80D9410", s.aim))
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.bor(bit32.band(p.z, 0xFFFF), height * 0x10000))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = anim, anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        local turret = s.turret
        if turret then
            turret.state.segments[s.slot] = nil
        end
        me:free()
        return
    end
    me:setup_collision(collision.piercing_break, collision.hits_navis, 1)
    me.collision:set_hit_effect(SPARK)
    me.collision.status_base = STATUS
    me.collision:set_region(nil)
    me.collision:present()
    me.timer = DELAY
    me:set_lifecycle("update")
end

-- `sub_80D9498`: after its delay it shows and arms.
local function wait(me: Object)
    me.collision:set_region(nil)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me.visible = true
    me:set_action(ARMED)
end

-- `sub_80D94B8`: its panel is hit.
local function armed(me: Object)
    me.collision:set_region(regions.single)
end

-- `sub_80D94C2`: spent, it arms again once its panel is free (and the
-- battle wasn't paused at its last update).
local function spent(me: Object)
    if battle.paused() ~= (me.state :: State).was_paused then
        return
    end
    me.collision:set_region(nil)
    if bit32.band(field.flags(me.panel_x, me.panel_y), OCCUPIED) == 0 then
        me.action = ARMED
    end
end

-- `sub_80D94E6`: ended by its turret.
local function ended(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80D9434`, unless paused: resolve the hits; it goes with the
-- battle's end; a hit spends it; and its action, unless dimmed.
local function tick(me: Object)
    if battle.paused() then
        return
    end
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
        if me.action ~= ENDED then
            me.action = SPENT
        end
    end
    if not battle.dimmed() then
        local a = me.action
        if a == WAIT then
            wait(me)
        elseif a == ARMED then
            armed(me)
        elseif a == SPENT then
            spent(me)
        elseif a == ENDED then
            ended(me)
        else
            error(string.format("Sensor's laser: action %#x reads past off_80D9488", a))
        end
    end
    c:present()
end

laser.kind = define.kind {
    id = "bn6:sensor/laser",
    pool = "attack",
    state = {
        turret = "object",
        slot = "u8",
        aim = "u8",
        was_paused = "bool",
    },
    -- `sub_80D9350`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        local paused = battle.paused()
        ;(me.state :: State).was_paused = paused
        if not paused then
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80D94F2`: a segment on panel (`x`, `y`) along `aim` for `turret`
-- (its element, damage word, side and facing), registered as its
-- segment `slot`. It runs while dimmed and paused.
function laser.spawn(turret: Object, x: number, y: number, aim: number, slot: number): Object?
    local o = battle.spawn(laser.kind, Vec3.zero)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.turret, s.slot = turret, slot
    turret.state.segments[slot] = o
    o.panel_x, o.panel_y = x, y
    o.element = turret.element
    o:set_damage_word(bit32.bor(turret.damage, bit32.lshift(turret.stamina, 16)))
    o.alliance, o.flip = turret.alliance, turret.flip
    s.aim = aim
    o.run_while_dimmed = true
    o.run_while_paused = true
    s.was_paused = battle.paused()
    return o
end

-- `sub_80D9532`: its turret ends it (its entry cleared): it goes at its
-- next unpaused, undimmed update.
function laser.stop(o: Object)
    local s = o.state :: State
    local turret = s.turret
    if turret then
        turret.state.segments[s.slot] = nil
    end
    o.action = ENDED
end

return laser
