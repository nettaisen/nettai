--!strict
-- A shock wave (the original's attack object #0x16, `sub_80C6B40`):
-- WaveArm's and PwrWave's, and the wave viruses send. It stands on its
-- panel for its variant's ticks, sends the next wave on, and goes when its
-- animation ends (or, for the quick variants, WaveArm's and PwrWave's, 4
-- ticks after sending the next). It needs a solid panel under it, and
-- stops hitting once it has hit something. A steering variant (WaveArm's)
-- turns toward the row of an enemy just ahead and to the side, once, and
-- keeps that row change. A variant that marks its panel (cracks it, breaks
-- it, poisons it) hits as a thrown attack.

local collision = require("../../rules/collision")
local panels = require("../../lib/panels")
local regions = require("../../lib/regions")

local wave = {}

-- A shock wave's look and timing (`ShockWaveVariant`, types.d.luau).
export type Variant = ShockWaveVariant

export type State = {
    variant: Variant,
    -- The row change it took: 0 none, 1 up, 2 down (Param2).
    turned: number,
}

function wave.variant(v: Variant): Variant
    return define.record("shock-wave-variant", v)
end

-- `byte_80C6B00`'s rows: 0 to 0xB the viruses' (no chip has them), 0xC to
-- 0xE WaveArm1's to WaveArm3's, 0xF PwrWave's.
wave.variants = {
    [0x0] = wave.variant { anim = 0, ticks = 22 },
    [0x1] = wave.variant { anim = 1, ticks = 16 },
    [0x2] = wave.variant { anim = 1, ticks = 11 },
    [0x3] = wave.variant { anim = 2, ticks = 6 },
    [0x4] = wave.variant { anim = 1, ticks = 16, panel = "cracked" },
    [0x5] = wave.variant { anim = 2, ticks = 6, panel = "poison" },
    [0x6] = wave.variant { anim = 0, ticks = 24 },
    [0x7] = wave.variant { anim = 1, ticks = 16 },
    [0x8] = wave.variant { anim = 1, ticks = 11 },
    [0x9] = wave.variant { anim = 2, ticks = 6 },
    [0xA] = wave.variant { anim = 1, ticks = 16 },
    [0xB] = wave.variant { anim = 2, ticks = 6 },
    [0xC] = wave.variant { anim = 1, ticks = 6, quick = true, steers = true },
    [0xD] = wave.variant { anim = 1, ticks = 6, quick = true, steers = true },
    [0xE] = wave.variant { anim = 1, ticks = 6, quick = true, steers = true },
    [0xF] = wave.variant { anim = 1, ticks = 6, quick = true },
}

local SPRITE = asset.sprite("shock-wave")
local SOUND = asset.sound("shock-wave")
local SOLID = 0x10
-- The rows ahead it looks for enemies in (`byte_80C6DB8`), by facing
-- (right, left) and panel column: the panels from the next one to the
-- field's edge (regions 0x08, 0x20, 0x07, 0x06, 0x02, 0x01).
local AHEAD_4 = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 } } }
local AHEAD_5 = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 } } }
local AHEAD_6 = define.region { panels = { { 0, 0 }, { 1, 0 }, { 2, 0 }, { 3, 0 }, { 4, 0 }, { 5, 0 } } }
local AHEAD: { [number]: { [number]: Region } } = {
    [0] = {
        [0] = AHEAD_6,
        AHEAD_5,
        AHEAD_4,
        regions.line,
        regions.long,
        regions.single,
        regions.single,
        regions.single,
    },
    [1] = {
        [0] = regions.single,
        regions.single,
        regions.single,
        regions.long,
        regions.line,
        AHEAD_4,
        AHEAD_5,
        AHEAD_6,
    },
}
-- Up and down a row (`dword_80C6DC8`), by the row change (1 up, 2 down;
-- the word's other bytes are 0), and where it looks for an enemy before
-- turning (regions 0x16 and 0x17).
local ROW_TURN = { -1, 1, 0, 0 }
local UP_AHEAD = define.region { panels = { { 0, 0 }, { 1, -1 } } }
local DOWN_AHEAD = define.region { panels = { { 0, 0 }, { 1, 1 } } }
-- Where it goes when it is done: x 240 pixels, off the field.
local OFF_FIELD = 0xF0

-- `sub_80C6CFC`: what it does to the panel it comes onto.
local function mark(x: number, y: number, panel: PanelType?)
    if panel == nil then
        return
    elseif panel == "cracked" then
        field.crack(x, y)
    elseif panel == "broken" then
        field.shatter(x, y)
    else
        field.set_type(x, y, panel)
    end
end

-- Off the field (X's whole pixels at 240, the fraction kept).
local function off_field(me: Object)
    local p = me.pos
    me.pos = Vec3.new(bit32.bor(OFF_FIELD * 0x10000, bit32.band(p.x, 0xFFFF)), p.y, p.z)
end

-- `sub_80C6D1E`: where a steering wave's next wave goes. Once it turned,
-- each next wave takes the same row change, where the row exists. Before
-- that: straight on if an enemy is in the row ahead; else up a row if an
-- enemy is up there ahead, else down a row likewise (the turn is then
-- kept); else straight on.
local function steer(me: Object): (number, number)
    local s = me.state :: State
    local x, y = me.panel_x + me.facing, me.panel_y
    local turned = s.turned
    if turned ~= 0 then
        local dy = ROW_TURN[turned]
        if dy == nil then
            error(string.format("shock wave row change %d reads past dword_80C6DC8", turned))
        end
        local row = y + dy
        if row == 0 or row > 3 then
            return x, y
        end
        return x, row
    end
    local ahead = AHEAD[bit32.bxor(me.alliance, me.flip)][me.panel_x]
    if ahead == nil then
        error(string.format("shock wave on column %d reads past byte_80C6DB8", me.panel_x))
    end
    if panels.random_enemy_panel(me, x, y, ahead) then
        return x, y
    end
    if y - 1 ~= 0 and panels.random_enemy_panel(me, x, y - 1, UP_AHEAD) then
        s.turned = 1
        return x, y - 1
    end
    if y + 1 <= 3 and panels.random_enemy_panel(me, x, y + 1, DOWN_AHEAD) then
        s.turned = 2
        return x, y + 1
    end
    return x, y
end

-- `sub_80C6C6A` (action 0): when its ticks are up, send the next wave on
-- and linger; out of sight at its animation's end.
local function roll(me: Object)
    if me.phase == 0 then
        me.phase = 1
    end
    local s = me.state :: State
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        local x, y
        if s.variant.steers then
            x, y = steer(me)
        else
            x, y = me.panel_x + me.facing, me.panel_y
        end
        wave.spawn(me, x, y, s.variant, me.damage + me.stamina * 0x10000, s.turned)
        me.action = 4
    end
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        off_field(me)
        me.collision:set_region(nil)
    end
end

-- `sub_80C6CBA` (action 4): gone at its animation's end, or (a quick one)
-- after its second timer.
local function linger(me: Object)
    if not (me.state :: State).variant.quick then
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
    else
        local t = me.timer2 - 1
        me.timer2 = t
        if t ~= 0 then
            return
        end
    end
    off_field(me)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80C6C14`: resolve its hits; it ends with the battle or when its
-- panel stops being solid, and stops hitting once it hit something.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or not bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local a = me.action
    if a == 0 then
        roll(me)
    elseif a == 4 then
        linger(me)
    else
        error(string.format("shock wave action %#x reads past its table", a))
    end
    c:present()
    me:highlight_collision_panels()
end

-- `sub_80C6B64`: on a solid panel of the field, or not at all.
local function init(me: Object)
    local x, y = me.panel_x, me.panel_y
    if not field.valid(x, y) or not bit32.btest(field.flags(x, y), SOLID) then
        me:free()
        return
    end
    local v = (me.state :: State).variant
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = v.anim
    me.anim_loaded = v.anim
    sprite:set_animation(v.anim)
    me.timer = v.ticks
    me.timer2 = 4
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    mark(x, y, v.panel)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    -- One that marks its panel breaks things too.
    me:setup_collision(if v.panel == nil then collision.attack else collision.thrown, collision.hits_navis, 3)
    local c = me.collision
    c:set_region(regions.single)
    c:set_hit_effect(nil)
    c:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

wave.kind = define.kind {
    id = "wavearm/wave",
    pool = "attack",
    state = { variant = "record:shock-wave-variant", turned = "u8" },
    -- `sub_80C6B40`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80C6CE4`: a wave of `variant` on (x, y) from `from` (a navi or the
-- wave before), with the damage word `damage`, having taken row change
-- `turned` (0 none). Its element is always null. Its position is the
-- spawner's registers (y, whatever, 0) until its init places it.
function wave.spawn(from: Object, x: number, y: number, variant: Variant, damage: number, turned: number?): Object?
    local o = battle.spawn(wave.kind, Vec3.new(y, 0, 0))
    if o then
        local s = o.state :: State
        s.variant, s.turned = variant, turned or 0
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = from.alliance, from.flip
        o.related1 = from
    end
    return o
end

return wave
