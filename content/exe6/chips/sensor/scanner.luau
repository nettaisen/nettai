--!strict
-- Sensor's scanner (the original's attack object #0x9F, `sub_80D9154`;
-- docs/engine/dimming-chips.md §8.3): a mark that steps along the turret's
-- aim a panel every 9 ticks, back to its start when blocked, and notes
-- when it stands on one of the other side's bodies. It registers in its
-- turret's `scanner` and clears it when it ends.

local scanner = {}

export type State = {
    -- Its turret.
    turret: Object?,
    -- Its aim (Param1): 0 up, 1 straight, 2 down; whether other bodies
    -- and neutral objects block it (Param2).
    aim: number,
    stops_at_objects: boolean,
    -- Where it started (+0x64, +0x65).
    start_x: number,
    start_y: number,
    -- It stood on one of the other side's bodies (+0x66).
    found: boolean,
}

local SPRITE = asset.sprite("erase-mark")
local STEP_SOUND = asset.sound("erase-man")
-- The aim's row step (`off_80D9328`).
local ROW_STEP: { [number]: number } = { [0] = -1, [1] = 0, [2] = 1 }
-- The other side's bodies (by its side: `off_80D9340`, `off_80D9344`);
-- the other side's other bodies and neutral objects (`dword_80D9348`,
-- `dword_80D934C`).
local OTHER_BODY: { [number]: number } = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
local BLOCKERS: { [number]: number } = { [0] = 0x0180_0000, [1] = 0x0280_0000 }

-- `sub_80D9298`: a step along its aim (true if blocked): with
-- `stops_at_objects`, not off a panel holding a blocker nor onto one; not
-- past the field's edge column (which leaves it where it is) nor onto row
-- 0 or 4 (which leaves it a column on).
local function step(me: Object): boolean
    battle.play_sound(STEP_SOUND)
    local s = me.state :: State
    local blockers = BLOCKERS[me.alliance]
    if s.stops_at_objects and bit32.band(field.flags(me.panel_x, me.panel_y), blockers) ~= 0 then
        return true
    end
    local dir = me.facing
    local edge = bit32.bxor(me.alliance, 1) * 5 + 1 + dir
    local x = me.panel_x + dir
    if x == edge then
        return true
    end
    me.panel_x = x
    local y = me.panel_y + ROW_STEP[s.aim]
    if y == 0 or y == 4 then
        return true
    end
    me.panel_y = y
    if s.stops_at_objects and bit32.band(field.flags(me.panel_x, me.panel_y), blockers) ~= 0 then
        return true
    end
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, bit32.band(me.pos.z, 0xFFFF))
    return false
end

-- `sub_80D9210`: a step every 9 ticks; blocked, back to its start.
local function move(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 8
        return
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    if step(me) then
        local s = me.state :: State
        me.panel_x, me.panel_y = s.start_x, s.start_y
        me:set_coordinates_from_panel()
        me.pos = Vec3.new(me.pos.x, me.pos.y, bit32.band(me.pos.z, 0xFFFF))
    end
    me.phase = 0
end

-- `sub_80D9178`: on its panel, shown, with its sound.
local function init(me: Object)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, bit32.band(me.pos.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local s = me.state :: State
    s.found = false
    battle.play_sound(STEP_SOUND)
    me:set_lifecycle("update")
end

-- `sub_80D91C8`: it goes with the battle's end; it notes the other side's
-- bodies (dimmed or not), and moves while not dimmed.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local s = me.state :: State
    if bit32.band(field.flags(me.panel_x, me.panel_y), OTHER_BODY[me.alliance]) ~= 0 then
        s.found = true
    end
    if battle.dimmed() then
        return
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("Sensor's scanner: action %d reads past off_80D920C", a))
    end
    move(me)
end

scanner.kind = define.kind {
    id = "sensor/scanner",
    pool = "attack",
    state = {
        turret = "object",
        aim = "u8",
        stops_at_objects = "bool",
        start_x = "u8",
        start_y = "u8",
        found = "bool",
    },
    -- `sub_80D9154`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite_while_paused()
    end,
}

-- `sub_80D924E`: a scanner from panel (`x`, `y`) along `aim` for `turret`
-- (its element, side and facing; no damage), registered as its scanner.
-- It acts while dimmed.
function scanner.spawn(turret: Object, x: number, y: number, aim: number, stops_at_objects: boolean): Object?
    local o = battle.spawn(scanner.kind, Vec3.zero)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.turret = turret
    turret.state.scanner = o
    o.panel_x, o.panel_y = x, y
    s.start_x, s.start_y = x, y
    s.aim, s.stops_at_objects = aim, stops_at_objects
    o.element = turret.element
    o:set_damage_word(0)
    o.alliance, o.flip = turret.alliance, turret.flip
    o.run_while_dimmed = true
    return o
end

-- It has stood on one of the other side's bodies.
function scanner.found(o: Object): boolean
    return (o.state :: State).found
end

-- `sub_80D9284`: it ends (its turret's entry cleared).
function scanner.stop(o: Object)
    local turret = (o.state :: State).turret
    if turret then
        turret.state.scanner = nil
    end
    o:set_lifecycle("destroy")
end

return scanner
