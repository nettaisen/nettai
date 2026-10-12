--!strict
-- HolyDrem's stand-in (EXE4's attack object #0x82, 0x080DD798; EXE4's own, the elder of EXE5's actor #0x1C): drawn as
-- the user, where the user stands, while the user vanishes (its status visuals kept). It fires a shot (./shot) along
-- the user's row at once; then it looks for holy panels over the whole field, from the far end's column toward the
-- user, rows 1 to 3 in each (0x080DD9AC, as EXE5's), and for each one, 10 ticks later, shows a flash on it, turns it
-- Normal and fires another shot. With none left it waits 60 ticks and gives the user back. Where EXE5's raises a
-- buster and fires from its attach point, EXE4's stands in animation 0x12 and fires from 40 pixels ahead, 20 up, with
-- the cannon's sound (0x8C) each time.

local shot = require("./shot")

local stand_in = {}

export type State = {
    -- The controller waiting on it (its +0x6C, the controller's flag).
    controller: Object?,
    -- The panel the scan is at (+0x60, +0x64), and the way it goes along the field (+0x68: toward the user).
    x: number,
    y: number,
    dx: number,
}

-- The flash on a holy panel the scan takes (effect row 0x45: 0c-13's animation 1).
local FLASH = new.effect { sprite = asset.sprite("grab-shot"), anim = 1, palette = 0 }
local SOUND = asset.sound("cannon")
-- Its pose (the user's sprite's animation 0x12).
local POSE = 0x12
-- Where a shot starts: 40 pixels toward the enemy and, by the navi (0x080DD940's row: 20 for navis 0 to 12, MegaMan's
-- among them), 20 up.
local MUZZLE_X, MUZZLE_Z = 40, 20
-- Ticks to the first shot, between the panels, and after the last.
local FIRST, BETWEEN, AFTER = 3, 10, 60

-- 0x080DD9AC: the next holy panel from the scan's place on (its place left there), or none.
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

-- 0x080DD898: the pose; the first shot in 3 ticks.
local function start(me: Object)
    me.anim = POSE
    me.timer = FIRST
    me.timer2 = 0
    me.phase = 4
end

-- 0x080DD8AC: when the time is up, the panel the scan found (after the first shot) flashes and turns Normal; a shot
-- 40 pixels ahead and 20 up, running while dimmed, with the cannon's sound; then the next holy panel in 10 ticks, or
-- with none, the end in 60.
local function fire(me: Object, s: State)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t ~= 0 then
        return
    end
    if me.timer2 ~= 0 and field.valid(s.x, s.y) then
        local cx, cy = field.center(s.x, s.y)
        battle.effect(Vec3.new(cx, cy, 0), FLASH)
        field.set_type(s.x, s.y, "normal")
    end
    local p = me.pos
    local muzzle = Vec3.new(int.i32(p.x + me.facing * MUZZLE_X * 0x1_0000), p.y, int.i32(p.z + MUZZLE_Z * 0x1_0000))
    local o = shot.spawn(me, muzzle)
    if o then
        o.run_while_dimmed = true
    end
    me.timer2 += 1
    battle.play_sound(SOUND)
    if next_holy(s) then
        me.timer = BETWEEN
    else
        me.timer = AFTER
        me.phase = 8
    end
end

-- 0x080DD950: 60 ticks; then what it wears comes off (0x0800B5DA) and it goes.
local function finish(me: Object)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t ~= 0 then
        return
    end
    me:remove_parts_of(me.related1 :: Object)
    me:set_lifecycle("destroy")
end

-- 0x080DD860: its one action (0x080DD878), by phase.
local function tick(me: Object)
    local s = me.state :: State
    if me.action ~= 0 then
        error(string.format("HolyDrem's stand-in: action %#x reads past its table (0x080DD874)", me.action))
    end
    local phase = me.phase
    if phase == 0 then
        start(me)
    elseif phase == 4 then
        fire(me, s)
    elseif phase == 8 then
        finish(me)
    else
        error(string.format("HolyDrem's stand-in: phase %#x reads past its table (0x080DD88C)", phase))
    end
end

-- 0x080DD7BC: drawn as the user (the sprite of its side's navi and soul, 0x0800B90A, with a shadow: navi 10's has
-- none, and no netbattle's navi is navi 10), where the user is, in the user's palette and facing, what the user's soul
-- puts on (0x0800B812); the user gone, its status visuals kept (`sub_80E1352` with 15); its way kept (+0x40); the scan
-- from the far end's column (6 for side 0, 1 for side 1), row 1, toward the user.
local function init(me: Object)
    local user = me.related1
    if not user then
        error("HolyDrem's stand-in has no user (its init reads through a null pointer)")
    end
    local sprite = me.sprite
    sprite:load_like(user)
    sprite.shadow = "ground"
    me.visible = true
    me.pos = user.pos
    me:set_panel_from_coordinates()
    sprite.palette = user.sprite.palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
    if user.actor_type == "player" then
        me:add_overlay_of(user)
    end
    dimming.hide_user_sparing(user)
    local s = me.state :: State
    s.dx = -me.facing
    s.y = 1
    s.x = 6 - 5 * me.alliance
    me:set_lifecycle("update")
    tick(me)
end

-- The destroy step (0x080DD96A): the user back, the controller on.
local function leave(me: Object)
    dimming.show_user(me.related1 :: Object)
    local c = (me.state :: State).controller
    if c and c.active then
        -- (The controller's `HolyDremState`: a type of another module, which the checker reads as any.)
        (c.state :: any).busy = false
    end
    me:free()
end

local kind = new.kind {
    id = "holydrem/stand-in",
    pool = "attack",
    state = { controller = "object", x = "i32", y = "i32", dx = "i32" },
    -- 0x080DD798.
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

-- 0x080DD97E: the stand-in for `user`, sent out by `controller` with the damage word, of the user's side alone (it
-- acts while dimmed). (Its column is the controller's register left 0: it starts on column 0 until its init places it
-- on the user.)
function stand_in.spawn(controller: Object, user: Object, damage: number): Object?
    local o = battle.spawn(kind)
    if o then
        o.panel_x, o.panel_y = 0, controller.panel_y
        o.element = controller.element
        o:set_damage_word(damage)
        o.related1 = user
        ;(o.state :: State).controller = controller
        o.alliance = user.alliance
        o.run_while_dimmed = true
    end
    return o
end

return stand_in
