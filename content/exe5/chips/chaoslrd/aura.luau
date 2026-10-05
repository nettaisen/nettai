--!strict
-- A part of the dark beast behind ChaosLrd (EXE5's effect object #0x47, 0x080E5A40; EXE5's own: EXE6 keeps its code as
-- leftover meteor code's neighbor): one of the beast's animations (sprite 08-14), 26 pixels down the field and up
-- from where it is put. It comes in (animation 13, the beast's body, out of a mosaic, fading in over 23 ticks, then
-- 26 more; the others hidden 35 ticks, then flashing in from white over 16), holds for its time, then blinks out
-- (shown when bit 1 of the ticks left is set). Its sprite steps every tick, dimmed or not.

export type State = {
    -- The beast's animation it shows (Param1).
    anim: number,
    -- How long it holds (Param3 as a halfword), and its blinking's ticks (Param2).
    hold: number,
    blink: number,
}

local aura = {}

local SPRITE = asset.sprite("chaos-beast")
-- The beast's body, which fades in rather than flashing in.
local BODY = 0x0D
local DOWN, UP = 26, 26

-- A halfword add to a 16.16 word's whole part.
local function whole_add(v: number, n: number): number
    local hi = bit32.band(bit32.rshift(v, 16) + n, 0xFFFF)
    return int.i32(bit32.bor(bit32.band(v, 0xFFFF), bit32.lshift(hi, 16)))
end

-- 0x080E5AD8: the body comes in: 30 ticks out of a mosaic (its size half the timer) and fading in (alpha 16 less
-- that), two ticks of timer a tick once it is at 16 or below; then 25 ticks; its phase byte counts the steps.
local function fade_in(me: Object)
    if me.phase ~= 2 then
        if me.phase ~= 1 then
            me.phase = 1
            me.timer = 0x1E
            me.visible = true
        end
        local half = bit32.rshift(me.timer, 1)
        -- `sprite_setMosaicSize(half, half)`: none at 0.
        me.sprite.mosaic = if half == 0 then nil else half
        me.sprite.alpha = 16 - half
        local t = me.timer
        if t <= 16 then
            t -= 1
        end
        t -= 1
        me.timer = int.u16(t)
        if t >= 0 then
            return
        end
        me.timer = 0x19
        me.phase = 2
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me.action = 4
    end
end

-- 0x080E5B2E: the others come in: hidden 35 ticks, then shown flashing from white (a color shader of twice the timer
-- in each channel) over 16; its phase byte counts the steps.
local function flash_in(me: Object)
    if me.phase ~= 2 then
        if me.phase ~= 1 then
            me.phase = 1
            me.timer = 0x22
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        me.visible = true
        me.timer = 0x0F
        me.phase = 2
    end
    local c = me.timer * 2
    me.sprite.color_shader = bit32.bor(0x8000, bit32.lshift(c, 10), bit32.lshift(c, 5), c)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me.action = 4
    end
end

-- Action 0 (0x080E5AC4).
local function come_in(me: Object)
    if me.anim == BODY then
        fade_in(me)
    else
        flash_in(me)
    end
end

-- Action 4 (0x080E5B7C): it holds for its time.
local function hold(me: Object)
    local s = me.state :: State
    s.hold = int.u16(s.hold - 1)
    if s.hold == 0 then
        me.action = 8
    end
end

-- Action 8 (0x080E5B8C): it blinks out (shown while bit 1 of its blinking's ticks is set); then it goes.
local function blink_out(me: Object)
    local s = me.state :: State
    me.visible = bit32.btest(s.blink, 2)
    s.blink = int.u8(s.blink - 1)
    if s.blink == 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = come_in, [4] = hold, [8] = blink_out }

-- 0x080E5AA0: its action; then its sprite steps.
local function tick(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("ChaosLrd's beast: action %#x reads past its table (0x080E5AB8)", me.action))
    end
    action(me)
    me.sprite:step()
end

-- 0x080E5A60: its sprite at its animation (a frame in, no shadow), facing its way; 26 pixels down the field and up;
-- its first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    local anim = (me.state :: State).anim
    me.anim, me.anim_loaded = anim, anim
    sprite:set_animation(anim)
    sprite:step()
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local p = me.pos
    me.pos = Vec3.new(p.x, whole_add(p.y, DOWN), whole_add(p.z, UP))
    me:set_lifecycle("update")
    tick(me)
end

aura.kind = define.kind {
    id = "chaoslrd/aura",
    pool = "effect",
    state = { anim = "u8", hold = "u16", blink = "u8" },
    -- 0x080E5A40.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080E5BB2: a part of `owner`'s (its side and facing) at `pos`: the beast's animation `anim`, holding `hold` ticks
-- and blinking out over `blink`.
function aura.spawn(owner: Object, pos: Vec3, anim: number, blink: number, hold: number): Object?
    local o = battle.spawn(aura.kind, pos)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.anim, s.hold, s.blink = anim, hold, blink
    end
    return o
end

return aura
