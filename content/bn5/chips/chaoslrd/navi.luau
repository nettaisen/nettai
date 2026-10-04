--!strict
-- ChaosLrd's Bass (BN5's actor object #0x51, 0x080C2EB8; BN5's own), brought by the navi chip controller (the summon
-- table's entry 29) to his side's back column, middle row. As he comes every field object's HP goes to 0, and if an
-- enemy navi stands in his side's two back columns around the middle row he leaves at once. Else the dark beast
-- rises behind him (five parts, ./aura) and stays a while; after 143 ticks flames gather (./gather) and he takes
-- shape (a glow, then Bass in his cape, flashing and then fading from dark violet as he drifts forward and up); 129
-- ticks on he raises his arm and throws the chaos strike (./strike) at the middle of the other side's area. 128
-- ticks on he goes in a puff, his cape with him, and the controller moves on.

local aura = require("./aura")
local gather = require("./gather")
local navi_chips = require("@common/navi-chips/navi")
local panels = require("../../rules/computer-navi/panels")
local strike = require("./strike")
local trajectory = require("../../lib/trajectory")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x48).
    controller: Object?,
    -- His color shader's channels (+0x60, +0x64, +0x68): red, green, blue.
    red: number,
    green: number,
    blue: number,
}

local chaoslrd = {}

local SPRITE = asset.sprite("bn5:bass")
local GLOW = asset.sprite("bn5:chaos-charge-glow")
local SOUND_APPEAR = asset.sound("bn5:appear")
local SOUND_GATHER = asset.sound("bn5:trap-vanish")
local SOUND_SHAPE = asset.sound("bn5:point-rise")
local SOUND_RAISE = asset.sound("bn5:chaos-lord")
local PUFF = define.effect { sprite = asset.sprite("bn5:puff"), anim = 0, palette = 0 }
-- His cape (0x080C38B6's word 0x10011308: a form overlay of his sprite, animations + 0x10, stepping while dimmed).
local CAPE: FormOverlaySpec = { sprite = SPRITE, stepping = "while_dimmed", anim_offset = 0x10 }
-- The navi chip controller calls his spawner (0x080C327E) through r3, which his position's Z takes; his init keeps
-- its fraction.
local SPAWNER_ADDRESS = 0x080C327F
-- What keeps him away (0x080C2FFC: an enemy navi's body, by side), and where it is looked for (0x080162A4's list 17:
-- his column and the one ahead, around the middle row).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
local AROUND = 17
-- The beast's parts (0x080C2F5C: animation, blinking ticks, holding ticks).
local PARTS: { { number } } = {
    { 0x0A, 0x0A, 0x5A },
    { 0x07, 0x0A, 0x5A },
    { 0x04, 0x0A, 0x5A },
    { 0x01, 0x0A, 0x5A },
    { 0x0D, 0x0A, 0x17C },
}
-- The flames (0x080C30C0: offset toward his front and up in pixels, palette).
local FLAMES: { { number } } = {
    { 0x30, -0x13, 1 },
    { 0x03, -0x16, 0 },
    { 0x0A, -0x34, 0 },
    { -0x06, -0x40, 0 },
    { -0x0E, -0x28, 0 },
    { -0x18, -0x0A, 1 },
    { 0x03, -0x03, 1 },
    { 0x16, -0x25, 1 },
    { 0x17, -0x4D, 1 },
    { 0x1F, -0x3A, 1 },
}
-- His first color (dark violet: red 5, green 1, blue 23), and white.
local RED, GREEN, BLUE = 5, 1, 0x17
local WHITE = 0x7FFF
-- His timers: the beast's (to the flames), the glow's (and when it shows), his shaping, his throw (and when he raises
-- his arm), his last.
local BEAST, GLOWING, GLOW_AT, SHAPING, THROWING, RAISE_AT, LAST = 0x8E, 0x25, 7, 0x48, 0x80, 0x6C, 0x28
-- While shaping: flashing from 52 left (its sound then), fading every 4 ticks from 36, drifting (his X velocity, a
-- 52nd of 32 pixels a tick toward his front; 3/4 of a pixel up a tick).
local FLASHING, FADING = 0x34, 0x24
local RISE = 0xC000

-- A halfword add to a 16.16 word's whole part.
local function whole_add(v: number, n: number): number
    local hi = bit32.band(bit32.rshift(v, 16) + n, 0xFFFF)
    return int.i32(bit32.bor(bit32.band(v, 0xFFFF), bit32.lshift(hi, 16)))
end

-- His side's back column (1 for a side facing right, 6 for one facing left).
local function back_column(me: Object): number
    return 5 * bit32.bxor(me.alliance, me.flip) + 1
end

-- His color shader of his channels.
local function shade(me: Object)
    local s = me.state :: State
    me.sprite.color_shader = bit32.bor(s.red, bit32.lshift(s.green, 5), bit32.lshift(s.blue, 10))
end

-- A timer's count down (a u16, read signed): whether it ran out (below 0).
local function count_down(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

-- Action 0 (0x080C303C): 143 ticks; then the flames gather on a point 8 pixels ahead of his column's middle panel and
-- 27 down the field, with their sound.
local function beast(me: Object)
    if not count_down(me) then
        return
    end
    local x, y = field.center(back_column(me), 2)
    local point = Vec3.new(int.i32(x + me.facing * 0x8_0000), int.i32(y + 0x1B_0000), 0)
    for _, f in FLAMES do
        gather.spawn(me, point, f[1], f[2], f[3])
    end
    battle.play_sound(SOUND_GATHER)
    me.timer = GLOWING
    me:set_action(4)
end

-- Action 4 (0x080C30E0): at 7 left the glow shows (14-1C, animation 1, 8 pixels up); 38 ticks; then he is there:
-- Bass on the ground (his first part hidden), in his cape, dark violet.
local function glow(me: Object)
    local sprite = me.sprite
    if me.timer == GLOW_AT then
        sprite:load(GLOW)
        -- sprite_noShadow: the first part is drawn with the sprite.
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 1, 1
        sprite:set_animation(1)
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, whole_add(p.z, 8))
        me.visible = true
    end
    if not count_down(me) then
        return
    end
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    -- `sub_8002FBE(0)`: his sprite's first part hidden.
    sprite.hidden_parts = bit32.bor(sprite.hidden_parts, 0x8000_0000)
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, whole_add(p.z, -8))
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.related1 = battle.form_overlay(me, CAPE)
    me.timer = SHAPING
    shade(me)
    me:set_action(8)
end

-- Action 8 (0x080C3184): 73 ticks: from 52 left he flickers (hidden when bit 1 of the timer is set, else white when
-- bit 2 is clear, else his color), with a sound at 52; below that he drifts forward and up, his color fading by 2
-- in each channel every fourth tick from 36.
local function shape(me: Object)
    local s = me.state :: State
    local t = me.timer
    if t >= FLASHING then
        if t == FLASHING then
            battle.play_sound(SOUND_SHAPE)
        end
        me.visible = true
        if bit32.btest(t, 2) then
            me.visible = false
        elseif not bit32.btest(t, 4) then
            me.sprite.color_shader = WHITE
        else
            shade(me)
        end
    else
        if t <= FADING and bit32.band(t, 3) == 0 then
            s.red = math.max(s.red - 2, 0)
            s.green = math.max(s.green - 2, 0)
            s.blue = math.max(s.blue - 2, 0)
            shade(me)
        end
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, int.i32(p.z + RISE))
    end
    if count_down(me) then
        me.timer = THROWING
        me:set_action(0x0C)
    end
end

-- Action 0xC (0x080C3230): at 108 left he raises his arm (animation 14) with a sound and throws the strike from where
-- he is (running while dimmed); 129 ticks; then animation 15.
local function throw(me: Object)
    if me.timer == RAISE_AT then
        me.anim = 0x0E
        battle.play_sound(SOUND_RAISE)
        local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        strike.spawn(me, me.pos, true, nil, word, true)
    end
    if count_down(me) then
        me.anim = 0x0F
        me.timer = LAST
        me:set_action(0x10)
    end
end

-- Action 0x10 (0x080C326E): 41 ticks; then he goes (the lifecycle byte alone, as the game stores it).
local function last(me: Object)
    if count_down(me) then
        me.lifecycle = "destroy"
    end
end

local ACTIONS = { [0] = beast, [4] = glow, [8] = shape, [0xC] = throw, [0x10] = last }

-- 0x080C3014.
local function act(me: Object)
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("ChaosLrd's Bass: action %#x reads past his table (0x080C3028)", me.action))
    end
    action(me)
end

-- 0x080C2F02: a sound; every field object's HP to 0 (0x080C32A2); with an enemy navi around his side's back middle
-- panel he goes at once (the lifecycle byte alone). Else the beast's parts 8 pixels ahead of that panel; his X
-- velocity and color; on that panel, 8 pixels ahead, 28 down the field and 18 up; his first update. (His sprite
-- loads later; the game decompresses 08-14 here, which only the drawing sees.)
local function init(me: Object)
    battle.play_sound(SOUND_APPEAR)
    for _, o in field.all_objects() do
        o.hp = 0
    end
    local side = bit32.bxor(me.alliance, me.flip)
    local back = back_column(me)
    local found = panels.random_offset(back, 2, ENEMY_NAVI[me.alliance], 0, AROUND, side)
    if found then
        me.lifecycle = "destroy"
        return
    end
    local x, y = field.center(back, 2)
    local at = Vec3.new(int.i32(x + me.facing * 0x8_0000), y, 0)
    for _, part in PARTS do
        aura.spawn(me, at, part[1], part[2], part[3])
    end
    me.timer = BEAST
    local vx = trajectory.div(0x20_0000, 0x34)
    me.vel = Vec3.new(int.mul32(vx, me.facing), me.vel.y, me.vel.z)
    local s = me.state :: State
    s.blue, s.red, s.green = BLUE, RED, GREEN
    me.panel_x, me.panel_y = back, 2
    me:set_coordinates_from_panel()
    local p = me.pos
    local z = bit32.bor(bit32.band(p.z, 0xFFFF), 0x12_0000)
    me.pos = Vec3.new(int.i32(p.x + me.facing * 0x8_0000), whole_add(p.y, 0x1C), int.i32(z))
    me:set_lifecycle("update")
    act(me)
end

-- 0x080C2EDC: a puff 16 pixels over him, his cape off, the controller moves on, and he is freed.
local function leave(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + 0x10_0000)), PUFF)
    local cape = me.related1
    if cape then
        cape:set_lifecycle("destroy")
    end
    local controller = (me.state :: State).controller
    if controller then
        navi_chip.navi_left(controller)
    end
    me:free()
end

chaoslrd.kind = define.kind {
    id = "bn5:chaoslrd/navi",
    pool = "actor",
    state = { controller = "object", red = "u8", green = "u8", blue = "u8" },
    -- 0x080C2EB8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            act(me)
        else
            leave(me)
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080C327E: ChaosLrd's Bass for `user`, reporting to `controller` (the summon table's entry 29).
function chaoslrd.summon(user: Object, controller: Object, spec: NaviChipSpec): Object?
    local o = navi_chips.spawn(chaoslrd.kind, user, spec, SPAWNER_ADDRESS)
    if not o then
        return nil
    end
    o.related1 = nil
    local s = o.state :: State
    s.controller = controller
    return o
end

return chaoslrd
