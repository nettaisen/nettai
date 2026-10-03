--!strict
-- The wide wave (the original's attack object #0x3B, `sub_80CB49C`): it
-- rises over the navi's column, and when its rising animation ends it
-- slides forward over a column of three panels. What differs between its
-- uses is its variant (`byte_80CB58C`'s speed and the routine's tests of
-- its kind): WideSht's ends on its first hit; SuprSpr's goes through,
-- hitting each panel once. See docs/engine/shot-chips.md §10.1.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local wave = {}

export type State = {
    variant: WaveVariant,
    -- Its hit modifier (Param2), for the variants that go through.
    hit_mod: number,
    -- The panel of its last hit, x | y << 8 (ExtraVars), and the count to
    -- its next afterimage (ExtraVars+4).
    hit_panel: number,
    trail: number,
}

-- A wave variant.
function wave.variant(v: WaveVariant): WaveVariant
    return define.record("wave-variant", v)
end

local SPRITE = asset.sprite("bn6:wide-wave")
local SOUND = asset.sound("bn6:wide-shot")
local HIT_MOD = 3
-- The rising wave's animation, and the sliding one's.
local ANIM_RISE, ANIM_SLIDE = 0, 1
-- A holding variant's wait after it rises.
local HOLD_TICKS = 0xF
-- The afterimages: every 5 ticks (the first after 10), of sprite "10-00"
-- (animation 1) with this color shader, for 20 ticks.
local TRAIL_FIRST, TRAIL_INTERVAL = 0xA, 5
local TRAIL_SPRITE = asset.sprite("bn6:rock-cubes")
local TRAIL_ANIM, TRAIL_SHADER, TRAIL_TICKS = 1, 0xC3FF, 0x14
-- What a holding variant writes over its spawner's attack (the step and
-- its entry flag, the element and the variant byte, which nothing reads,
-- as one word): at its spawn, its first update, and its end.
local HOLD_SPAWN, HOLD_INIT, HOLD_END = 2, 3, 0

local function panel_word(me: Object): number
    return me.panel_x + me.panel_y * 0x100
end

-- A holding variant's word over its spawner's attack variables.
local function hold(me: Object, word: number)
    local owner = me.related1
    if owner then
        owner.step, owner.step_init = word, 0
        owner.attack_element = 0
    end
end

local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80CB628`: rise until the animation ends; then over the next column
-- (a holding variant waits there 15 ticks); then slide.
local function move(me: Object, v: WaveVariant)
    local init = me.phase_init
    if init == 0 then
        if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
            return
        end
        me.anim = ANIM_SLIDE
        me.panel_x = me.panel_x + me.facing
        me:set_coordinates_from_panel()
        me.collision:set_region(regions.wide)
        if v.holds then
            me.timer = HOLD_TICKS
            me.phase_init = 1
        else
            me.phase_init = 2
        end
    else
        local slide = init == 2
        if init == 1 then
            local left = bit32.band(me.timer - 1, 0xFFFF)
            me.timer = left
            if left == 0 then
                me.phase_init = 2
                slide = true
            end
        end
        if slide then
            local p = me.pos
            me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
            me:set_panel_from_coordinates()
        end
    end
    me:update_collision_panels()
end

-- `sub_80CB6B0`: a trailing variant's afterimage every 5 ticks.
local function trail(me: Object, s: State)
    if not s.variant.trails then
        return
    end
    local left = s.trail - 1
    s.trail = left
    if left > 0 then
        return
    end
    s.trail = TRAIL_INTERVAL
    battle.afterimage(me, me.pos, {
        sprite = TRAIL_SPRITE,
        anim = TRAIL_ANIM,
        flip = bit32.bxor(me.alliance, me.flip),
        lifetime = TRAIL_TICKS,
        color_shader = TRAIL_SHADER,
    })
end

-- `sub_80CB5B4`: the hits against it resolve and spark; the battle's end
-- ends it, and so does a hit, unless it goes through (then its region is
-- off until it reaches another panel); it moves, and off the field it
-- ends.
local function tick(me: Object)
    local s = me.state :: State
    local v = s.variant
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        finish(me)
        return
    end
    if c.hit_flags ~= 0 then
        if not v.pierces then
            finish(me)
            return
        end
        c:set_region(nil)
        s.hit_panel = panel_word(me)
    end
    move(me, v)
    trail(me, s)
    if me.phase_init ~= 0 and c.region == nil and panel_word(me) ~= s.hit_panel then
        c:set_region(regions.wide)
    end
    if field.valid(me.panel_x, me.panel_y) then
        c:present()
    else
        finish(me)
    end
end

-- `sub_80CB4D4`: rising over the navi's column, unseen to hits until it
-- moves; its first update; the sound.
local function init(me: Object)
    local s = me.state :: State
    local v = s.variant
    local sprite = me.sprite
    sprite:load(v.sprite or SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = ANIM_RISE, ANIM_RISE
    sprite:set_animation(ANIM_RISE)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.vel = Vec3.new(me.facing * v.speed, me.vel.y, me.vel.z)
    me.panel_x = me.panel_x - me.facing
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    local hit_mod = if v.pierces then s.hit_mod else HIT_MOD
    me:setup_collision(v.collision, collision.hits_navis, hit_mod)
    local c = me.collision
    c:set_region(nil)
    c:set_hit_effect(nil)
    c:present()
    if v.holds then
        hold(me, HOLD_INIT)
    end
    local bug = v.bug
    if bug then
        c.bugs = bug
    end
    local palette = v.palette
    if palette then
        sprite.palette = palette
    end
    s.trail = TRAIL_FIRST
    me:set_lifecycle("update")
    tick(me)
    battle.play_sound(SOUND)
end

wave.kind = define.kind {
    id = "bn6:widesht/wave",
    pool = "attack",
    state = { variant = "record:wave-variant", hit_mod = "u8", hit_panel = "u32", trail = "i32" },
    -- `sub_80CB49C`.
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
            -- `sub_80CB4C0`.
            if (me.state :: State).variant.holds then
                hold(me, HOLD_END)
            end
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80CB686`: a wave of `variant` from `owner` on panel (x, y) at
-- height `z` (16.16), with `element`, the damage word and the hit
-- modifier a piercing variant uses; nil if the attack pool is full. It
-- takes the owner's side and flip, runs later this tick, right after its
-- spawner, and keeps its Z.
function wave.fire(owner: Object, x: number, y: number, z: number, variant: WaveVariant, element: number, damage: number, hit_mod: number): Object?
    local o = battle.spawn(wave.kind, Vec3.new(0, 0, z))
    if o then
        local s = o.state :: State
        s.variant = variant
        s.hit_mod = hit_mod
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        if variant.holds then
            o.related1 = owner
            hold(o, HOLD_SPAWN)
        end
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- The variants (the original's kinds 0 to 9: its speed, 16.16 pixels a
-- tick, by `byte_80CB58C`). Kinds 0 to 2 hold their spawner and wait
-- before sliding, and kind 9 trails afterimages and bugs what it hits;
-- nothing in the game fires those.
local function v(speed: number, more: { pierces: boolean?, holds: boolean?, trails: boolean? }?): WaveVariant
    local m: { pierces: boolean?, holds: boolean?, trails: boolean? } = more or {}
    return wave.variant {
        speed = speed,
        pierces = m.pierces or false,
        holds = m.holds or false,
        trails = m.trails or false,
        collision = if m.trails then collision.attack_480 else collision.attack,
        -- `sub_801A4D0(0x14, 2)` and palette 1.
        bug = if m.trails then 0x214 else nil,
        palette = if m.trails then 1 else nil,
    }
end
local ROWS: { [number]: WaveVariant } = {
    [0] = v(0x3_0000, { holds = true }),
    [1] = v(0x3_8000, { holds = true }),
    [2] = v(0x4_0000, { holds = true }),
    [3] = v(0x6_0000),
    [4] = v(0x6_8000),
    [5] = v(0x7_0000),
    [6] = v(0x6_0000, { pierces = true }),
    [7] = v(0x6_8000, { pierces = true }),
    [8] = v(0x7_0000, { pierces = true }),
    [9] = v(0x7_0000, { pierces = true, trails = true }),
}
wave.variants = {
    -- WideSht's (kind 3).
    wide = ROWS[3],
    -- SuprSpr's (kind 6): it goes through.
    super = ROWS[6],
}

-- The variant of kind `n`, for the callers that name one by number; past
-- the table the game reads on into the next routine's code: an error.
function wave.by_number(n: number): WaveVariant
    local r = ROWS[n]
    if not r then
        error(string.format("wave kind %#x reads past byte_80CB58C", n))
    end
    return r
end

return wave
