--!strict
-- The ghosts DarkNeo's Bass leaves as he flies to the middle of the field (EXE4's effect object #0x47, 0x080E6F94;
-- ./navi sends out four, 0x080CB552): an animation of Bass's sheet (08-19: his body's or his cape's) tinted, drawn as
-- a mark on the floor's is (its shadow part hidden), where he took off, at his height. It waits its delay, then
-- flies his flight after him (`ghost.flight`, the panels he went and the fall a tick he had), and goes when it ends.
-- All along it is hidden one tick in three. It runs while dimmed.

local ghost = {}

type State = {
    -- Its animation (Param1), the ticks before it flies (Param2), its blink's count (Param3) and its tint (its
    -- +0x60).
    anim: number,
    delay: number,
    blink: number,
    shader: number,
}

local SPRITE = asset.sprite("bass")
-- (Its palette goes by its Bass's first parameter, a navi chip's level: 3, an SP's, 7; any other but 0, 8. DarkNeo's
-- is 0, and no other chip brings this Bass.)
local PALETTE = 0

local WAITING, FLYING = 0x0, 0x4

-- 0x080C4A44: how much of a flight of six ticks is left with `ticks` to go (the table 0x080C4A88 by the flight's
-- length, the object's second timer: 3, 9, 18, 27, 33 and 36 thirty-sixths gone), as a 16.16 fraction. (Its other
-- tables, for flights of 8 and 12 ticks and the rest, aren't this Bass's.)
local GONE = { 3, 9, 18, 27, 33, 36 }
function ghost.flight(ticks: number): number
    local gone = GONE[ticks]
    if gone == nil then
        error(string.format("a flight with %d ticks to go reads past its table (0x080C4A88)", ticks))
    end
    return 0x1_0000 - int.tdiv(gone * 0x1_0000, 36)
end

-- Where a flier whose velocity holds the panels to go (X, Y) is with `left` of the flight to go: from its panel, 40
-- pixels a column and 24 a row.
function ghost.position(me: Object, left: number): (number, number)
    local cx, cy = field.center(me.panel_x, me.panel_y)
    return int.i32(cx + me.vel.x * left * 40), int.i32(cy + me.vel.y * left * 24)
end

-- Shown but one tick in three.
local function blink(me: Object, s: State)
    me.visible = true
    local n = s.blink - 1
    if n == 0 then
        n = 3
        me.visible = false
    end
    s.blink = n
end

-- 0x080E704C: its delay.
local function wait(me: Object, s: State)
    blink(me, s)
    local n = s.delay - 1
    s.delay = int.u8(n)
    if n == 0 then
        me:set_action(FLYING)
    end
end

-- 0x080E707C: the flight, falling as Bass fell.
local function fly(me: Object, s: State)
    blink(me, s)
    local x, y = ghost.position(me, ghost.flight(me.timer))
    me.pos = Vec3.new(x, y, int.i32(me.pos.z - me.vel.z))
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t == 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080E7026: nothing while dimmed for one that doesn't fall (none: every ghost of DarkNeo's falls).
local function tick(me: Object)
    if me.vel.z == 0 and battle.dimmed() then
        return
    end
    local s = me.state :: State
    if me.action == WAITING then
        wait(me, s)
    elseif me.action == FLYING then
        fly(me, s)
    else
        error(string.format("DarkNeo's ghost: action %#x reads past its table (0x080E7044)", me.action))
    end
end

-- 0x080E6FB8: on its panel; Bass's sheet, its animation asked for, its shadow on the ground and the rest drawn as the
-- shadow is, the shadow part itself hidden; its palette and tint; facing its side's way; shown; one with no delay
-- flies at once.
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = s.anim
    sprite:set_animation(s.anim)
    sprite.shadow = "ground"
    sprite.under_objects = true
    sprite.hidden_parts = bit32.bor(sprite.hidden_parts, 0x8000_0000)
    sprite.palette = PALETTE
    sprite.color_shader = s.shader
    sprite:set_flip(me.alliance)
    s.blink = 2
    me.visible = true
    me:set_lifecycle("update")
    if s.delay == 0 then
        me.action = FLYING
    end
end

ghost.kind = new.kind {
    id = "darkneo/ghost",
    pool = "effect",
    state = { anim = "u8", delay = "u8", blink = "u8", shader = "u16" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080E7100 (0x080E70D6): a ghost of `bass` in `anim`, tinted `shader`, flying after `delay` ticks: where he is (his
-- panel and height), with the flight he has ahead (his timers and the panels to go) and `fall` a tick; running while
-- dimmed. (Its position is the spawner's registers but for the height, until its init places it.)
function ghost.spawn(bass: Object, anim: number, delay: number, shader: number, fall: number): Object?
    local o = battle.spawn(ghost.kind, Vec3.new(0, 0, bass.pos.z))
    if o then
        o.related1 = bass
        o.alliance = bass.alliance
        o.panel_x, o.panel_y = bass.panel_x, bass.panel_y
        o.timer, o.timer2 = bass.timer, bass.timer2
        o.vel = Vec3.new(bass.vel.x, bass.vel.y, fall)
        o.run_while_dimmed = true
        local s = o.state :: State
        s.anim, s.delay, s.shader = anim, delay, shader
    end
    return o
end

return ghost
