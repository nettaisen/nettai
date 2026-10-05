--!strict
-- HolyDrem's stand-in (EXE5's actor object #0x1C, 0x080BD728; EXE5's own, built as EXE6's dimming stand-ins are:
-- @exelib/dimming/stand_in): drawn as the user, where the user stands, with the buster raised, while the user
-- vanishes. It fires a shot (./shot) along the user's row at once; then it looks for holy panels over the whole
-- field, from the far end's column toward the user, rows 1 to 3 in each (0x080BD96C), and for each one, 10 ticks
-- later, shows a flash on it, turns it Normal and fires another shot. With none left it waits 60 ticks, lowers the
-- buster and gives the user back (docs/design/exe5-map.md §6.2). The shots come from the user's attach point 29.
-- (The panel it reads is the engine's holy type, whichever game's stage or chip made it.)

local buster = require("../../lib/buster")
local shot = require("./shot")
local stand_in = require("@exelib/dimming/stand_in")

local actor = {}

export type State = {
    -- The controller waiting on it (@exelib/dimming/stand_in).
    controller: Object?,
    -- The buster it holds (its +0x74).
    held: Object?,
    -- The panel the scan is at (+0x60, +0x64), and the way it goes along the field (+0x68: toward the user).
    x: number,
    y: number,
    dx: number,
    -- The shots fired (+0x22).
    shots: number,
}

-- The flash on a holy panel the scan takes (effect 41: 0c-13's animation 1).
local FLASH = define.effect { sprite = asset.sprite("grab-shot"), anim = 1, palette = 0 }
-- The pose (the user's sprite's animation 14); the shots' attach point.
local POSE, MUZZLE = 14, 29
-- Ticks to the first shot, between the panels, and after the last.
local FIRST, BETWEEN, AFTER = 3, 10, 60

-- 0x080BD96C: the next holy panel from the scan's place on (its place left there), or none.
local function next_holy(s: State): boolean
    while field.valid(s.x, s.y) do
        local p = field.panel(s.x, s.y)
        if p and p.kind == "holy" then
            return true
        end
        s.y += 1
        if s.y > 3 then
            s.y = 1
            s.x += s.dx
        end
    end
    return false
end

-- 0x080BD85C: the pose and the buster up (held, stepping while dimmed); the first shot in 3 ticks.
local function start(me: Object, s: State)
    me.anim = POSE
    buster.raise_arm_for(me.related1 :: Object, me, "held", true)
    me.timer = FIRST
    s.shots = 0
    me.phase = 4
end

-- 0x080BD87A: when the time is up, the panel the scan found (after the first shot) flashes and turns Normal; a
-- shot from the attach point; then the next holy panel in 10 ticks, or with none, the end in 60.
local function fire(me: Object, s: State)
    local user = me.related1 :: Object
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t ~= 0 then
        return
    end
    if s.shots ~= 0 and field.valid(s.x, s.y) then
        local cx, cy = field.center(s.x, s.y)
        battle.effect(Vec3.new(cx, cy, 0), FLASH)
        field.set_type(s.x, s.y, "normal")
    end
    -- sub_8018810 by the user's identity, with the stand-in's side and facing (the user's).
    local dx, dz = user:attach_point(MUZZLE)
    local p = me.pos
    local o = shot.spawn(me, Vec3.new(p.x + int.shl(dx, 16), p.y, p.z + int.shl(dz, 16)))
    if o then
        o.run_while_dimmed = true
    end
    s.shots += 1
    if next_holy(s) then
        me.timer = BETWEEN
    else
        me.timer = AFTER
        me.phase = 8
    end
end

-- 0x080BD908: 60 ticks; then the buster goes, the user's parts come off it, and it goes.
local function finish(me: Object, s: State)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t ~= 0 then
        return
    end
    s.held = nil
    me:remove_parts_of(me.related1 :: Object)
    me:set_lifecycle("destroy")
end

-- 0x080BD826, by phase (its one action, 0x080BD83C).
local function tick(me: Object)
    local s = me.state :: State
    if me.action ~= 0 then
        error(string.format("HolyDrem's stand-in: action %#x reads past its table (0x080BD838)", me.action))
    end
    local phase = me.phase
    if phase == 0 then
        start(me, s)
    elseif phase == 4 then
        fire(me, s)
    elseif phase == 8 then
        finish(me, s)
    else
        error(string.format("HolyDrem's stand-in: phase %#x reads past its table (0x080BD850)", phase))
    end
end

-- 0x080BD74C: drawn as the user (stand_in.init: its sprite with a shadow, palette, facing and parts, where the
-- user stands, the user gone); its facing kept (+0x40); the scan from the far end's column (6 for side 0, 1 for
-- side 1), row 1, toward the user.
local function init(me: Object)
    stand_in.init(me)
    local s = me.state :: State
    me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
    s.dx = -me.facing
    s.y = 1
    s.x = 6 - 5 * me.alliance
    me:set_lifecycle("update")
    tick(me)
end

-- The destroy step (0x080BD92A): the user back, the controller on.
local function leave(me: Object)
    dimming.show_user(me.related1 :: Object)
    local c = (me.state :: State).controller
    if c and c.active then
        -- (`stand_in.Waiting`: a type of another module, which the checker
        -- reads as any.)
        (c.state :: any).busy = false
    end
    me:free()
end

actor.kind = define.kind {
    id = "holydrem/actor",
    pool = "actor",
    state = { controller = "object", held = "object", x = "i32", y = "i32", dx = "i32", shots = "u16" },
    -- 0x080BD728.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            leave(me)
            return
        end
        if me.active then
            me:update_sprite()
        end
    end,
}

return actor
