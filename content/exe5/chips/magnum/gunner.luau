--!strict
-- Magnum's gunner (EXE5's actor object #2, 0x080B8CA8; EXE5's own): a stand-in for the user (@exelib/dimming/stand_in's
-- drawing) that moves a sight over the enemy's area, a step every 7 ticks, until the user presses A or 180 ticks
-- pass; then, half a second later, it shoots the sight's panels, breaking them. Its mode (the controller's Param1;
-- Magnum's 0) picks how the sight moves and what it shoots: 0, column by column across the field to the next one
-- with an enemy panel, wrapping round, and the column's three panels; 1, row by row up the enemy's area, and the
-- row's three panels there; 2, as 0 but backward, every 5 ticks.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local stand_in = require("@exelib/dimming/stand_in")

local gunner = {}

export type State = {
    -- The chip's controller (its Param2, the stage it counts to 2: fired, then gone).
    controller: Object?,
    -- Its mode (Param1).
    mode: number,
    -- The sight's column (ExtraVars) and row (+4), its step's ticks (+8), and the enemy's direction (+0x40).
    x: number,
    y: number,
    interval: number,
    dir: number,
}

-- What the controller keeps: the gunner's stage (its Param2).
type Waiting = { stage: number }

local AIM_TICKS, AFTER_AIM, FIRE_TICKS, AFTER_FIRE = 180, 30, 3, 30
local FIRE_ANIM = 12
-- The sight's step by mode (0x080B8DE8).
local INTERVALS = { [0] = 7, [1] = 7, [2] = 5 }
local SOUND_STEP, SOUND_SHOT, SOUND_BLAST = asset.sound("erase-man"), asset.sound("hockey-puck"), asset.sound("target-move")
local SOUND_BREAK = asset.sound("panel-crack")
-- The sight (effect 0x2C) and the explosion on each shot panel (effect 0).
local SIGHT = define.effect { sprite = asset.sprite("sight"), anim = 0, palette = 0 }
local EXPLOSION = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
-- A column the sight stops on has a panel of the other side's (0x080B8F64, 0x080B9048: by side, require and forbid).
local ENEMY_PANEL = {
    [0] = { require = 0x10020, forbid = 0x0 },
    [1] = { require = 0x10000, forbid = 0x20 },
}
-- Where a sweep starts over (0x080B8F74 forward, 0x080B9058 backward), and the row mode's first column (0x080B8FD0,
-- 0x080B9110), by side.
local WRAP_FORWARD, WRAP_BACKWARD = { [0] = 0, [1] = 7 }, { [0] = 7, [1] = 0 }
local ROW_X = { [0] = 6, [1] = 3 }

local function stage_up(me: Object, to: number?)
    local c = (me.state :: State).controller
    if c and c.active then
        local w = c.state :: Waiting
        w.stage = if to then to else w.stage + 1
    end
end

-- 0x080B8F76: column `x` has an enemy panel (rows 3 to 1).
local function has_enemy(me: Object, x: number): boolean
    local e = ENEMY_PANEL[me.alliance]
    for y = 3, 1, -1 do
        if field.check(x, y, e.require, e.forbid) then
            return true
        end
    end
    return false
end

-- The sight on (x, y), for the step's ticks.
local function sight(me: Object, s: State, x: number, y: number)
    local cx, cy = field.center(x, y)
    local o = battle.effect(Vec3.new(cx, cy, 0), SIGHT)
    if o then
        o.timer = s.interval
    end
end

-- The sight on column `x`'s three panels.
local function sight_column(me: Object, s: State, x: number)
    for y = 3, 1, -1 do
        sight(me, s, x, y)
    end
end

-- 0x080B8EFC: on to the next column with an enemy panel (none round the field: it stays).
local function step_forward(me: Object, s: State)
    local from = s.x
    -- (Bounded: with no enemy panel anywhere the original loops forever.)
    for _ = 1, 64 do
        s.x += s.dir
        if s.x - 1 < 0 or s.x - 1 >= 6 then
            s.x = WRAP_FORWARD[me.alliance]
        elseif s.x == from then
            return
        elseif has_enemy(me, s.x) then
            sight_column(me, s, s.x)
            return
        end
    end
    error("Magnum's sight finds no column (the original's sweep never ends)")
end

-- 0x080B8F92: the next row up (from 3 again), the enemy's three columns.
local function step_row(me: Object, s: State)
    local y = s.y - 1
    if y <= 0 then
        y = 3
    end
    s.y = y
    local x = ROW_X[me.alliance]
    for k = 0, 2 do
        sight(me, s, x - k, y)
    end
end

-- 0x080B8FD2: back to the previous column with an enemy panel, from the far end once off the field.
local function step_backward(me: Object, s: State)
    s.x -= s.dir
    if not (s.x - 1 >= 0 and s.x - 1 < 6 and has_enemy(me, s.x)) then
        s.x = WRAP_BACKWARD[me.alliance]
        while true do
            s.x -= s.dir
            if s.x - 1 < 0 or s.x - 1 >= 6 then
                return
            end
            if has_enemy(me, s.x) then
                break
            end
        end
    end
    sight_column(me, s, s.x)
end

-- A shot on (x, y): its explosion, its hit (thrown, hitting navis, the heavy spark, hit modifier 3), and the panel
-- broken (`breaks`) with a crack's sound if it broke.
local function shot(me: Object, x: number, y: number, breaks: (x: number, y: number) -> boolean)
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), EXPLOSION)
    -- (`sub_80C53A6`'s counterpart: a hit that runs while the battle is dimmed.)
    local h = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.heavy,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
    })
    if h then
        h.run_while_dimmed = true
    end
    if breaks(x, y) then
        battle.play_sound(SOUND_BREAK)
    end
end

-- 0x080B905A (modes 0 and 2) and 0x080B90AA (mode 1): the sight's panels; the camera shakes.
local function shoot(me: Object, s: State)
    if s.mode == 1 then
        local x = ROW_X[me.alliance]
        for k = 0, 2 do
            shot(me, x - k, s.y, field.break_empty)
        end
    else
        for y = 3, 1, -1 do
            shot(me, s.x, y, function(px, py)
                return field.break_panel(px, py)
            end)
        end
    end
    battle.shake_camera(1, 30)
end

-- 0x080B8DA4: aim, by phase.
local function aim(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- 0x080B8DC4.
        me.timer = AIM_TICKS
        s.interval = INTERVALS[s.mode]
        me.timer2 = 1
        battle.play_sound(SOUND_STEP)
        me.phase = 4
    elseif phase == 4 then
        -- 0x080B8DEC: a step every interval; fire on A or once the time is up.
        local t2 = me.timer2 - 1
        me.timer2 = bit32.band(t2, 0xFFFF)
        if t2 == 0 then
            me.timer2 = s.interval
            if s.mode == 0 then
                step_forward(me, s)
            elseif s.mode == 1 then
                step_row(me, s)
            elseif s.mode == 2 then
                step_backward(me, s)
            else
                error(string.format("Magnum's sight: mode %d reads past its table", s.mode))
            end
            battle.play_sound(SOUND_STEP)
        end
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t == 0 or (me.related1 :: Object):dimmed_pressed("a") then
            me.timer = AFTER_AIM
            me.phase = 8
        end
    elseif phase == 8 then
        -- 0x080B8E3C: half a second; it has fired.
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t == 0 then
            stage_up(me, 1)
            me.action = 4
            me.phase, me.phase_init = 0, 0
        end
    else
        error(string.format("Magnum's gunner: aim phase %#x reads past its table", phase))
    end
end

-- 0x080B8E56: fire, by phase.
local function fire(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- 0x080B8E74.
        me.anim = FIRE_ANIM
        me.timer = FIRE_TICKS
        battle.play_sound(SOUND_SHOT)
        me.phase = 4
    elseif phase == 4 then
        -- 0x080B8E8E.
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t == 0 then
            shoot(me, s)
            battle.play_sound(SOUND_BLAST)
            me.timer = AFTER_FIRE
            me.phase = 8
        end
    elseif phase == 8 then
        -- 0x080B8EC8: then its parts off and gone; the user comes back at its next update.
        local t = me.timer - 1
        me.timer = bit32.band(t, 0xFFFF)
        if t == 0 then
            me:remove_parts_of(me.related1 :: Object)
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("Magnum's gunner: fire phase %#x reads past its table", phase))
    end
end

gunner.kind = define.kind {
    id = "magnum/gunner",
    pool = "actor",
    state = {
        controller = "object",
        mode = "u8",
        x = "i32",
        y = "i32",
        interval = "i32",
        dir = "i32",
    },
    -- 0x080B8CA8.
    update = function(me: Object)
        local s = me.state :: State
        local l = me.lifecycle
        if l == "init" then
            -- 0x080B8CCC: drawn as the user, who vanishes.
            stand_in.init(me)
            -- (`object_getEnemyDirection`: by side, whatever its facing.)
            s.dir = 1 - 2 * me.alliance
            me:set_lifecycle("update")
        elseif l ~= "update" then
            -- 0x080B8EE6: the user back; it is gone.
            dimming.show_user(me.related1 :: Object)
            stage_up(me)
            me:free()
            return
        end
        local a = me.action
        if a == 0 then
            aim(me, s)
        elseif a == 4 then
            fire(me, s)
        else
            error(string.format("Magnum's gunner: action %#x reads past its table", a))
        end
        me:update_sprite()
    end,
}

-- 0x080B9112: a gunner of mode `mode` for `user` on `controller`'s panel, with its element and the damage word
-- `damage`, of the user's side and facing; it runs while the battle is dimmed.
function gunner.spawn(controller: Object, user: Object, damage: number, mode: number): Object?
    local o = battle.spawn(gunner.kind)
    if o then
        o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
        o.element = controller.element
        o:set_damage_word(damage)
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.controller, s.mode = controller, mode
    end
    return o
end

return gunner
