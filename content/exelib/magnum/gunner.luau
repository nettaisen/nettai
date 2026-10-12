--!strict
-- Magnum's gunner (EXE5's actor object #2, 0x080B8CA8; EXE4's attack object #0x50, 0x080D756C), made for a game's look
-- (`gunner.make`): a stand-in for the user (@exelib/dimming/stand_in's drawing) that moves a sight over the enemy's
-- area, a step every 7 ticks, until the user presses A or 180 ticks pass; then, half a second later, it shoots the
-- sight's panels, breaking them. The sight moves column by column to one with an enemy panel (the game's `step`), and
-- it shoots the column's three panels: its mode 0 (the controller's Param1, Magnum's). (Modes 1, row by row up the
-- enemy's area, and 2, as 0 but backward every 5 ticks, no chip has.) The games' differ in their pool, what the
-- stand-in wears, their sounds, the firing animation, the sight's effect row and how the sight steps.

local regions = require("@exelib/regions")
local stand_in = require("@exelib/dimming/stand_in")

local gunner = {}

export type State = {
    -- The chip's controller (its Param2, the stage it counts to 2: fired, then gone).
    controller: Object?,
    -- The sight's column (ExtraVars), its step's ticks (+8), and the enemy's direction (+0x40).
    x: number,
    interval: number,
    dir: number,
}

export type Look = {
    -- The kind's id and pool (EXE5's an actor, EXE4's an attack).
    id: string,
    pool: "actor" | "attack",
    -- What it wears of its user (EXE5's a player's overlay, `stand_in.overlay`; EXE4's nothing).
    wear: (me: Object, user: Object) -> (),
    -- Its sounds: a step of the sight, the shot, the blast and a panel broken.
    step_sound: Sound,
    shot: Sound,
    blast: Sound,
    crack: Sound,
    -- Its animation as it fires (EXE5's 12, EXE4's 0x10).
    fire_anim: number,
    -- The sight on a panel (EXE5's effect row 0x2C, EXE4's 0x49: 10-22) and the explosion on each shot panel (row 0).
    sight: Effect,
    explosion: Effect,
    -- Its hit's spark (the heavy one), collision type and what it hits.
    spark: Spark,
    is: CollisionType,
    hits: CollisionType,
    -- A step of the sight (`gunner.sweep`, EXE5's; `gunner.next_or_wrap`, EXE4's): `sight_column` puts the sight on a
    -- column's panels.
    step: (me: Object, s: State, sight_column: (x: number) -> ()) -> (),
}

export type Gunner = {
    kind: Kind,
    -- A gunner for `user` on `controller`'s panel, with its element and the damage word `damage`, of the user's side
    -- and facing; it runs while the battle is dimmed (EXE5's 0x080B9112, EXE4's 0x080D798E).
    spawn: (controller: Object, user: Object, damage: number) -> Object?,
}

-- What the controller keeps: the gunner's stage (its Param2).
type Waiting = { stage: number }

local AIM_TICKS, AFTER_AIM, FIRE_TICKS, AFTER_FIRE = 180, 30, 3, 30
-- The sight's step (EXE5's 0x080B8DE8, EXE4's 0x080D766C, by its mode: 0's).
local INTERVAL = 7

-- A column the sight stops on has a panel of the other side's (EXE5's 0x080B8F64, EXE4's 0x080D77E0: by side).
local function enemy_panel(x: number, y: number, side: number): boolean
    if side == 0 then
        return field.check(x, y, 0x10020, 0x0)
    end
    return field.check(x, y, 0x10000, 0x20)
end

-- Where a sweep starts over (EXE5's 0x080B8F74, EXE4's 0x080D77F0), by side.
local function wrap(side: number): number
    return if side == 0 then 0 else 7
end

-- EXE5's 0x080B8F76, EXE4's 0x080D77F2: column `x` has an enemy panel (rows 3 to 1).
local function has_enemy(me: Object, x: number): boolean
    for y = 3, 1, -1 do
        if enemy_panel(x, y, me.alliance) then
            return true
        end
    end
    return false
end

-- EXE5's step (0x080B8EFC): on to the next column with an enemy panel, wrapping round the field; none round the field:
-- it stays.
function gunner.sweep(me: Object, s: State, sight_column: (x: number) -> ())
    local from = s.x
    -- (Bounded: with no enemy panel anywhere the original loops forever.)
    for _ = 1, 64 do
        s.x += s.dir
        if s.x - 1 < 0 or s.x - 1 >= 6 then
            s.x = wrap(me.alliance)
        elseif s.x == from then
            return
        elseif has_enemy(me, s.x) then
            sight_column(s.x)
            return
        end
    end
    error("Magnum's sight finds no column (the original's sweep never ends)")
end

-- EXE4's step (0x080D776E): the next column if it has an enemy panel; else from the field's start on to the first
-- that has one; none: the sight off the field, past its far edge, and no sight drawn.
function gunner.next_or_wrap(me: Object, s: State, sight_column: (x: number) -> ())
    s.x += s.dir
    if s.x - 1 >= 0 and s.x - 1 < 6 and has_enemy(me, s.x) then
        sight_column(s.x)
        return
    end
    s.x = wrap(me.alliance)
    while true do
        s.x += s.dir
        if s.x - 1 < 0 or s.x - 1 >= 6 then
            return
        end
        if has_enemy(me, s.x) then
            sight_column(s.x)
            return
        end
    end
end

local function stage_up(me: Object, to: number?)
    local c = (me.state :: State).controller
    if c and c.active then
        local w = c.state :: Waiting
        w.stage = if to then to else w.stage + 1
    end
end

function gunner.make(look: Look): Gunner
    -- The sight on column `x`'s three panels, for the step's ticks.
    local function sight_column_of(s: State): (x: number) -> ()
        return function(x: number)
            for y = 3, 1, -1 do
                local cx, cy = field.center(x, y)
                local o = battle.effect(Vec3.new(cx, cy, 0), look.sight)
                if o then
                    o.timer = s.interval
                end
            end
        end
    end

    -- A shot on (x, y): its explosion, its hit (hit modifier 3), and the panel broken with a crack's sound if it broke.
    local function shot(me: Object, x: number, y: number)
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0), look.explosion)
        -- (EXE5's `sub_80C53A6` counterpart: a hit that runs while the battle is dimmed.)
        local h = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = look.spark,
            target = look.hits,
            self_type = look.is,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 3,
        })
        if h then
            h.run_while_dimmed = true
        end
        if field.break_panel(x, y) then
            battle.play_sound(look.crack)
        end
    end

    -- EXE5's 0x080B905A, EXE4's 0x080D78D6: the sight's column; the camera shakes.
    local function shoot(me: Object, s: State)
        for y = 3, 1, -1 do
            shot(me, s.x, y)
        end
        battle.shake_camera(1, 30)
    end

    -- EXE5's 0x080B8DA4, EXE4's 0x080D7628: aim, by phase.
    local function aim(me: Object, s: State)
        local phase = me.phase
        if phase == 0 then
            me.timer = AIM_TICKS
            s.interval = INTERVAL
            me.timer2 = 1
            battle.play_sound(look.step_sound)
            me.phase = 4
        elseif phase == 4 then
            -- A step every interval; fire on A (EXE4 reads its side's joypad record, the same navi's) or once the time
            -- is up.
            local t2 = me.timer2 - 1
            me.timer2 = bit32.band(t2, 0xFFFF)
            if t2 == 0 then
                me.timer2 = s.interval
                look.step(me, s, sight_column_of(s))
                battle.play_sound(look.step_sound)
            end
            local t = me.timer - 1
            me.timer = bit32.band(t, 0xFFFF)
            if t == 0 or (me.related1 :: Object):dimmed_pressed("a") then
                me.timer = AFTER_AIM
                me.phase = 8
            end
        elseif phase == 8 then
            -- Half a second; it has fired.
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

    -- EXE5's 0x080B8E56, EXE4's 0x080D76DE: fire, by phase.
    local function fire(me: Object, s: State)
        local phase = me.phase
        if phase == 0 then
            me.anim = look.fire_anim
            me.timer = FIRE_TICKS
            battle.play_sound(look.shot)
            me.phase = 4
        elseif phase == 4 then
            local t = me.timer - 1
            me.timer = bit32.band(t, 0xFFFF)
            if t == 0 then
                shoot(me, s)
                battle.play_sound(look.blast)
                me.timer = AFTER_FIRE
                me.phase = 8
            end
        elseif phase == 8 then
            -- Then what it wears off (EXE4's wears nothing) and gone; the user comes back at its next update.
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

    local kind = new.kind {
        id = look.id,
        pool = look.pool,
        state = {
            controller = "object",
            x = "i32",
            interval = "i32",
            dir = "i32",
        },
        update = function(me: Object)
            local s = me.state :: State
            local l = me.lifecycle
            if l == "init" then
                -- Drawn as the user, who vanishes.
                stand_in.init(me, look.wear)
                -- (`object_getEnemyDirection`: by side, whatever its facing.)
                s.dir = 1 - 2 * me.alliance
                me:set_lifecycle("update")
            elseif l ~= "update" then
                -- The user back; it is gone.
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

    local function spawn(controller: Object, user: Object, damage: number): Object?
        local o = battle.spawn(kind)
        if o then
            o.panel_x, o.panel_y = controller.panel_x, controller.panel_y
            o.element = controller.element
            o:set_damage_word(damage)
            o.related1 = user
            o.alliance, o.flip = user.alliance, user.flip
            o.run_while_dimmed = true
            local s = o.state :: State
            s.controller = controller
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return gunner
