--!strict
-- Magnum's gunner (the original's actor object #0x14, `sub_80BB2A0`;
-- docs/engine/dimming-chip-effects.md §11): a stand-in for the user
-- (@exelib/dimming/stand_in) that moves a sight over the enemy's area, a step
-- every 12 ticks (across the columns on the middle row, then down the far
-- column's middle), until the user presses A or 300 ticks pass; then it
-- shoots the sight's three panels, breaking them.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local stand_in = require("@exelib/dimming/stand_in")

local gunner = {}

export type State = {
    -- The chip's controller (@exelib/dimming/stand_in).
    controller: Object?,
    -- The far column (ExtraVars+0xC), the enemy's nearest (+0x10), the
    -- sight's panel (+0x14, +0x18), and its mode when it fired (+0x1C).
    far_x: number,
    near_x: number,
    x: number,
    y: number,
    fired_in_rows: boolean,
}

-- `byte_80BB3F4`: the far column by side.
local FAR_X: { [number]: number } = { [0] = 6, [1] = 1 }
local TIMEOUT, STEP = 300, 12
local FIRE_TICKS, FIRE_ANIM, FIRE_ANIM_AT, AFTER = 0x10, 0xC, 4, 0x3C
-- (The cursor steps with the sound EraseMan's mark has, the original's
-- 0x10E.)
local SOUND_STEP = asset.sound("erase-man")
local SOUND_SHOT = asset.sound("hockey-puck")
local SOUND_BLAST = asset.sound("target-move")
-- The sight (effect 0x49), each part 12 ticks: its panel and the column's
-- other rows (24 pixels up and down) or the row's other columns (40
-- pixels either side).
local SIGHT = define.effect { sprite = asset.sprite("sight"), anim = 0, palette = 0 }
-- The shot's panels (hit regions 4 and 0x1D, not turned by the facing):
-- the column's three, or the row's three.
local COLUMN = { { 0, 0 }, { 0, -1 }, { 0, 1 } }
local ROW = { { 0, 0 }, { 1, 0 }, { -1, 0 } }
-- Each panel's hit (`dword_80BB5F4`): thrown, hitting navis, hit modifier
-- 3, a heavy spark.
local HIT_IS, HIT_HITS, HIT_MOD = collision.thrown, collision.hits_navis, 3

-- `sub_800D5F0`: how many panels of column `x` aren't `side`'s.
local function foreign_panels(x: number, side: number): number
    local n = 0
    for y = 1, 3 do
        local p = field.panel(x, y)
        if p and p.alliance ~= side then
            n += 1
        end
    end
    return n
end

-- `sub_80BB59A`: the sight on its panel, 12 ticks.
local function draw(me: Object, s: State)
    local x, y = field.center(s.x, s.y)
    local parts = if me.phase_init == 0
        then { { x, y }, { x, y - 0x18_0000 }, { x, y + 0x18_0000 } }
        else { { x, y }, { x - 0x28_0000, y }, { x + 0x28_0000, y } }
    for _, p in parts do
        local o = battle.effect(Vec3.new(p[1], p[2], 0), SIGHT)
        if o then
            o.timer = STEP
        end
    end
end

-- `sub_80BB4CE`: the sight's next step (the phase's init flag is its mode):
-- across the middle row to the far column, then down its rows, then back.
local function step(me: Object, s: State)
    local d = me.facing
    if me.phase_init == 0 then
        if s.x == s.far_x then
            s.x = s.far_x - d
            s.y = 1
            me.phase_init = 4
        else
            s.x += d
        end
    elseif me.phase_init == 4 then
        if s.y >= 3 then
            s.x = s.near_x
            s.y = 2
            me.phase_init = 0
        else
            s.y += 1
        end
    else
        error(string.format("Magnum's sight: mode %#x reads past off_80BB4F0", me.phase_init))
    end
    draw(me, s)
end

-- `sub_80BB3F6`: step the sight every 12 ticks; fire on A or after 300
-- ticks.
local function aim(me: Object, s: State)
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 ~= 0 then
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            me.timer = STEP
            step(me, s)
            battle.play_sound(SOUND_STEP)
        end
        if not (me.related1 :: Object):dimmed_pressed("a") then
            return
        end
    end
    s.fired_in_rows = me.phase_init ~= 0
    me.phase, me.phase_init = 8, 0
end

-- `sub_80BB52E`: a hit on each of the sight's panels, which break (or
-- crack under something), with an explosion; the camera shakes.
local function shoot(me: Object, s: State)
    for _, d in if s.fired_in_rows then ROW else COLUMN do
        local x, y = s.x + d[1], s.y + d[2]
        local h = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = sparks.heavy,
            target = HIT_HITS,
            self_type = HIT_IS,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if h then
            h.run_while_dimmed = true
        end
        field.break_panel(x, y)
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, 0), effects.explosion)
    end
    battle.play_sound(SOUND_BLAST)
    battle.shake_camera(1, 0x1E)
end

-- `sub_80BB42E`: 17 ticks, the throw's animation 4 before the shot.
local function fire(me: Object, s: State)
    if me.phase_init == 0 then
        me.timer = FIRE_TICKS
        me.phase_init = 4
    end
    me.timer2 = int.u16(me.timer2 - 1)
    if me.timer == FIRE_ANIM_AT then
        me.anim = FIRE_ANIM
    end
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    battle.play_sound(SOUND_SHOT)
    shoot(me, s)
    me.timer = AFTER
    me.phase, me.phase_init = 0xC, 0
end

-- `sub_80BB46A`: 61 ticks, then the user comes back.
local function done(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        stand_in.leave(me)
    end
end

-- `sub_80BB398`: the sight on the enemy's nearest column's middle panel.
local function start(me: Object, s: State)
    local d = me.facing
    s.y = 2
    s.far_x = FAR_X[me.alliance]
    local x = s.far_x
    while foreign_panels(x, me.alliance) ~= 0 do
        x -= d
    end
    x += d
    s.near_x, s.x = x, x
    me.phase_init = 0
    draw(me, s)
    battle.play_sound(SOUND_STEP)
    me.timer2 = TIMEOUT
    me.timer = STEP
    me.phase, me.phase_init = 4, 0
    aim(me, s)
end

-- `sub_80BB35C`: shown (unless the local player is blind to it), by phase.
local function tick(me: Object)
    stand_in.show(me)
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        start(me, s)
    elseif phase == 4 then
        aim(me, s)
    elseif phase == 8 then
        fire(me, s)
    elseif phase == 0xC then
        done(me)
    else
        error(string.format("Magnum's gunner: phase %#x reads past its table", phase))
    end
end

gunner.kind = define.kind {
    id = "magnum/gunner",
    pool = "actor",
    state = {
        controller = "object",
        far_x = "i32",
        near_x = "i32",
        x = "i32",
        y = "i32",
        fired_in_rows = "bool",
    },
    -- `sub_80BB2A0`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80BB2C4: drawn as the user, who vanishes; on at once.
            stand_in.init(me)
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

return gunner
