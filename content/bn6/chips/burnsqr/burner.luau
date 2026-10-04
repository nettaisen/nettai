--!strict
-- BurnSqr's burner (the original's actor object #0x13, `sub_80BAF50`;
-- docs/engine/dimming-chip-effects.md §2): a stand-in for the user
-- (@exelib/dimming/stand_in) that moves a 2x2 square over the whole field, a
-- step every 6 ticks, until the user presses A or 300 ticks pass, then
-- sets the square's solid panels on fire (flames: chips/heatman/flame).

local stand_in = require("@exelib/dimming/stand_in")
local collision = require("../../rules/collision")
local heat_flame = require("../../chips/heatman/flame")

local burner = {}

export type State = {
    -- The chip's controller, waiting on it (@exelib/dimming/stand_in).
    controller: Object?,
    -- The square: its start column (ExtraVars+0xC), where it turns
    -- (+0x10), and its top left panel (+0x14, +0x18).
    start_x: number,
    turn_x: number,
    x: number,
    y: number,
    -- The square was highlighted this tick (`sub_80BB1D8`), which leaves
    -- the top row in the register the flames' height comes from.
    highlighted: boolean,
}

-- `dword_80BB094`: the start column and the turning column by side.
local START_X: { [number]: number } = { [0] = 5, [1] = 2 }
local TURN_X: { [number]: number } = { [0] = 1, [1] = 6 }
-- Ticks to fire (unless A comes first), and between the square's steps.
local TIMEOUT, STEP = 300, 6
-- Firing: the throw's animation 4 ticks before the flames; then the user
-- comes back 61 ticks later.
local FIRE_TICKS, FIRE_ANIM, FIRE_ANIM_AT, AFTER = 0x10, 0xC, 4, 0x3C
-- (The cursor steps with the sound EraseMan's mark has, the original's
-- 0x10E.)
local SOUND_STEP = asset.sound("bn6:erase-man")
local SOUND_FIRE = asset.sound("bn6:mark")
-- The square's corners (effect 0x48), each 6 ticks: 40 pixels apart
-- across, 24 down.
local CORNER = define.effect { sprite = asset.sprite("bn6:hit-sparks"), anim = 12, palette = 0 }
local ACROSS, DOWN = 0x28_0000, 0x18_0000
-- Its flames (`sub_80C8DE0`): fire, HeatMan's (acting while dimmed), 30
-- ticks.
local FIRE = 1
local FLAME: HeatFlame = { while_dimmed = true, ticks = 0x1E, is = collision.attack }
local SOLID = 0x10

-- `sub_80BB23E`: the square's four corners at its panels' centers.
local function draw(me: Object, s: State)
    local d = me.facing
    local x, y = field.center(s.x, s.y)
    for _, p in { { x, y }, { x + ACROSS * d, y }, { x + ACROSS * d, y + DOWN }, { x, y + DOWN } } do
        local o = battle.effect(Vec3.new(p[1], p[2], 0), CORNER)
        if o then
            o.timer = 6
        end
    end
end

-- `ho_80BB170`: the square's next step (the phase's init flag holds where
-- it is in its round): down a row, back to the turning column, up a row,
-- forward to the start column.
local function step(me: Object, s: State)
    local d = me.facing
    local at = me.phase_init
    if at == 0 then
        s.y = 2
        me.phase_init = 4
    elseif at == 4 then
        s.x -= d
        if s.x == s.turn_x then
            me.phase_init = 8
        end
    elseif at == 8 then
        s.y = 1
        me.phase_init = 0xC
    elseif at == 0xC then
        s.x += d
        if s.x == s.start_x then
            me.phase_init = 0
        end
    else
        error(string.format("BurnSqr's square: step %#x reads past jt_80BB18C", at))
    end
    draw(me, s)
end

-- `sub_80BB098`: step the square every 6 ticks; fire on A or after 300
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
    battle.play_sound(SOUND_FIRE)
    me.phase, me.phase_init = 8, 0
end

-- `sub_80BB0D2`: the square blinks (lit while Timer2's bit 2 is clear,
-- `sub_80BB1D8`) for 17 ticks, then burns.
local function fire(me: Object, s: State)
    if me.phase_init == 0 then
        me.timer = FIRE_TICKS
        me.phase_init = 4
    end
    s.highlighted = not bit32.btest(me.timer2, 4)
    if s.highlighted then
        local dx = 1 - 2 * me.alliance
        for _, y in { s.y, s.y + 1 } do
            for _, x in { s.x, s.x + dx } do
                if x >= 1 and x <= 6 and y >= 1 and y <= 3 then
                    field.highlight(x, y)
                end
            end
        end
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
    -- sub_80BB1FE: a flame on each solid panel of the square. Its height's
    -- fraction is r3: the top row the highlight left there, or, without
    -- the highlight this tick, what the object update loop left (nothing
    -- on the way sets it).
    local d = me.facing
    local z = if s.highlighted then s.y else battle.loop_register()
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    for _, p in { { s.x, s.y }, { s.x + d, s.y }, { s.x + d, s.y + 1 }, { s.x, s.y + 1 } } do
        if bit32.btest(field.flags(p[1], p[2]), SOLID) then
            heat_flame.spawn(me, p[1], p[2], FIRE, FLAME, damage, Vec3.new(p[2], FIRE, z))
        end
    end
    me.timer = AFTER
    me.phase, me.phase_init = 0xC, 0
end

-- `sub_80BB10C`: 61 ticks, then the user comes back.
local function done(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        stand_in.leave(me)
    end
end

-- `sub_80BB048`: the square at its start, drawn; 300 ticks to fire.
local function start(me: Object, s: State)
    s.start_x = START_X[me.alliance]
    s.x = s.start_x
    s.turn_x = TURN_X[me.alliance]
    s.y = 1
    draw(me, s)
    battle.play_sound(SOUND_STEP)
    me.timer2 = TIMEOUT
    me.timer = STEP
    me.phase, me.phase_init = 4, 0
    aim(me, s)
end

-- `sub_80BB00C`: shown (unless the local player is blind to it), by phase.
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
        error(string.format("BurnSqr's burner: phase %#x reads past off_80BB038", phase))
    end
end

burner.kind = define.kind {
    id = "bn6:burnsqr/burner",
    pool = "actor",
    state = {
        controller = "object",
        start_x = "u32",
        turn_x = "u32",
        x = "i32",
        y = "i32",
        highlighted = "bool",
    },
    -- `sub_80BAF50`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80BAF74: drawn as the user, who vanishes; on at once.
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

return burner
