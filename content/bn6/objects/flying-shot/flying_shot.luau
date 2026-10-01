--!strict
-- The flying shot (the original's attack object #0xB, `sub_80C60A8`):
-- TrnArrw's arrows, the Falzar Beast buster's shots, a thrown obstacle. It
-- flies forward at a steady speed (and falls, if fired falling) until it
-- has passed its range in panel centers, and ends on the first thing it
-- hits or when it leaves the field. What differs between its uses is its
-- variant, a record (a row of `byte_80C6038`): its collision, element,
-- spark, look, speed, range and status. It may wait some ticks before it
-- sets off, and takes a palette. See docs/engine/objects-and-player.md
-- §B8.

local absorbed_obstacle = require("../absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local element = require("../../lib/element")
local sparks = require("../../lib/sparks")

local flying_shot = {}

-- A flying shot's variant (`FlyingShotVariant`, types.d.luau).
export type Variant = FlyingShotVariant

-- How a shot is fired: the ticks it waits before it sets off, whether it
-- falls, its palette, and for a thrown obstacle, the obstacle (kind |
-- animation << 4).
export type Options = { wait: number?, falls: boolean?, palette: number?, obstacle: number? }

export type State = {
    variant: Variant,
    -- Ticks left to wait (Param2), whether it falls (Param3), its palette
    -- and then its range left (Param4).
    wait: number,
    falls: boolean,
    range: number,
    -- A thrown obstacle's kind and animation (kind | animation << 4), its
    -- spawner's word (the game's ExtraVars).
    obstacle: number,
}

function flying_shot.variant(v: Variant): Variant
    return define.record("flying-shot-variant", v)
end

-- `byte_80C6038`'s rows.
flying_shot.variants = {
    -- Row 0: a sword-like wave (no user ported).
    wave = flying_shot.variant {
        collision = { is = collision.piercing_break_thrown, hits = collision.hits_navis },
        hit_mod = 3,
        element = "null",
        spark = sparks.null,
        sprite = asset.sprite("sword"),
        speed = 0x5_0000,
        range = 8,
    },
    -- Row 1: a faster one with a heavy spark (no user ported).
    heavy_wave = flying_shot.variant {
        collision = { is = collision.attack, hits = collision.hits_navis },
        hit_mod = 3,
        element = "null",
        spark = sparks.heavy,
        sprite = asset.sprite("sword"),
        speed = 0xA_0000,
        range = 8,
    },
    -- Row 2: TrnArrw's arrow: an aqua hit whose spark shows over the
    -- panel's center, twanging as it sets off.
    arrow = flying_shot.variant {
        collision = { is = collision.attack, hits = collision.hits_navis },
        hit_mod = 1,
        element = "aqua",
        spark = sparks.aqua,
        sprite = asset.sprite("bow"),
        anim = 3,
        speed = 0x6_0000,
        range = 8,
        panel_spark = true,
        launch_sound = asset.sound("twang"),
    },
    -- Row 3: the Falzar Beast buster's shot.
    beast_shot = flying_shot.variant {
        collision = { is = collision.attack, hits = collision.hits_navis },
        hit_mod = 0,
        element = "null",
        spark = sparks.impact,
        sprite = asset.sprite("beast-shot"),
        speed = 0xC_0000,
        range = 8,
    },
    -- Row 4: a slower one (no user ported).
    slow_beast_shot = flying_shot.variant {
        collision = { is = collision.attack, hits = collision.hits_navis },
        hit_mod = 1,
        element = "null",
        spark = sparks.impact,
        sprite = asset.sprite("beast-shot"),
        speed = 0x8_0000,
        range = 8,
    },
    -- Row 5: the Falzar Beast buster's volley shots: two panels' range,
    -- ending in a burst (effect 7).
    volley_shot = flying_shot.variant {
        collision = { is = collision.attack, hits = collision.hits_navis },
        hit_mod = 0,
        element = "null",
        spark = sparks.impact,
        sprite = asset.sprite("beast-shot"),
        speed = 0xC_0000,
        range = 2,
        end_effect = define.effect { sprite = asset.sprite("copy-mark"), anim = 0, palette = 0 },
    },
    -- Row 6: a thrown obstacle (the buster's throw): it looks like the
    -- obstacle, breaks, and lights its panel.
    thrown = flying_shot.variant {
        collision = { is = collision.breaking, hits = collision.hits_navis },
        hit_mod = 3,
        element = "null",
        secondary = { "break" },
        spark = sparks.breaking,
        sprite = asset.sprite("beast-shot"),
        speed = 0xA_0000,
        range = 8,
        highlight = true,
        obstacle = true,
    },
}

-- How fast it falls when fired falling, 16.16 a tick.
local FALL = -0x1_2000
-- A thrown obstacle with a sprite of this index keeps its parts' own
-- facing (`sub_8002EAC`), and is not flipped.
local UNFLIPPED_INDEX = 0x23
-- The height of a panel spark (`sub_801A100`), 16.16.
local PANEL_SPARK_Z = 0x10_0000
-- Collision results that show a hit spark (`object_spawnCollisionEffect`),
-- unless the first bit is set too.
local SPARKS, NO_SPARK = 0x3F800000, 1

-- Its end: not drawn, its hit region cleared (`object_clearCollisionRegion`);
-- the next update frees it.
local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_800E6E8`: `c` lies past `a` and up to `b`, either way.
local function crossed(a: number, b: number, c: number): boolean
    if a <= b then
        return c > a and c <= b
    end
    return c > b and c <= a
end

-- `sub_801A100`: the hit spark over the center of the panel it is over
-- (`sub_800E258`), 16 pixels up (one RNG draw when it shows).
local function panel_spark(me: Object, v: Variant)
    local f = me.collision.hit_flags
    if not bit32.btest(f, SPARKS) or bit32.btest(f, NO_SPARK) or v.spark == nil then
        return
    end
    local p = me.pos
    local x = int.tdiv(int.asr(p.x, 16) + 0xA0, 0x28)
    local y = int.tdiv(int.asr(p.y, 16) + 0x20, 0x18)
    local cx, cy = field.center(x, y)
    battle.spark(me, battle.jitter(0xF, Vec3.new(cx, cy, PANEL_SPARK_Z)), v.spark)
end

-- Fly (and fall); each panel center passed spends its range, and it ends
-- when that runs out (leaving its variant's end effect) or it leaves the
-- field.
local function fly(me: Object, s: State)
    local p, vel = me.pos, me.vel
    local cx = field.center(me.panel_x, me.panel_y)
    local x = int.i32(p.x + vel.x)
    me.pos = Vec3.new(x, p.y, p.z + vel.z)
    if crossed(x, p.x, cx) then
        local left = s.range - 1
        s.range = left
        if left <= 0 then
            local effect = s.variant.end_effect
            if effect then
                me:set_coordinates_from_panel()
                battle.effect(me.pos, effect)
            end
            finish(me)
            return
        end
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        finish(me)
    end
end

-- `sub_80C619C`: the hits against it resolve and spark; it ends on a hit,
-- else waits out its wait (with its variant's launch sound as the wait
-- ends) or flies. A highlighting variant lights its panel.
local function tick(me: Object)
    local s = me.state :: State
    local v = s.variant
    local c = me.collision
    c:remove()
    if v.panel_spark then
        panel_spark(me, v)
    else
        me:hit_spark()
    end
    if c.hit_flags ~= 0 then
        finish(me)
    else
        local wait = s.wait
        if wait ~= 0 then
            wait -= 1
            s.wait = wait
            local sound = v.launch_sound
            if wait == 0 and sound then
                battle.play_sound(sound)
            end
        else
            fly(me, s)
        end
    end
    c:present()
    -- `object_highlightCurrentCollisionPanels`: its region is its own
    -- panel until it ends (none). Drawn only.
    if v.highlight and c.region ~= 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
end

-- `sub_80C60CC`: its look, element, range, speed and collision from its
-- variant (a thrown obstacle's look from the obstacle), on the panel under
-- it. (The game overwrites its link to its owner with its record's
-- address, so the owner is forgotten.)
local function init(me: Object)
    local s = me.state :: State
    local v = s.variant
    me.related1 = nil
    local look = me.sprite
    local anim: number = v.anim or 0
    local unflipped = false
    if v.obstacle then
        -- The absorbed obstacle's sprite (a number until the absorbed
        -- obstacles' kinds are definitions).
        local kind = bit32.band(s.obstacle, 0xF)
        local sprite = absorbed_obstacle.sprite(kind)
        look:load(sprite)
        anim = bit32.band(bit32.rshift(s.obstacle, 4), 0xF)
        -- The sprite's index: the part of "CC-II" after the dash.
        unflipped = tonumber(string.sub(sprite, 4), 16) == UNFLIPPED_INDEX
    else
        look:load(v.sprite)
    end
    look.shadow = if v.shadow then "ground" else "with_sprite"
    me.anim = anim
    me.anim_loaded = 0xFF
    me.visible = true
    if not unflipped then
        -- (A thrown obstacle that keeps its parts' own facing, `sub_8002EAC`,
        -- is not flipped. Drawn only.)
        look:set_flip(bit32.bxor(me.alliance, me.flip))
    end
    -- (Its palette is the fourth spawn parameter, which then holds its
    -- range.)
    look.palette = s.range
    me.element = element.byte(v.element, v.secondary)
    s.range = v.range
    me:set_panel_from_coordinates()
    me.vel = Vec3.new(me.facing * v.speed, me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(v.collision.is, v.collision.hits, v.hit_mod)
    local c = me.collision
    c:set_hit_effect(v.spark)
    local status = v.status or 0
    if status ~= 0 then
        c.status_base = status
    end
    c:present()
    me.vel = Vec3.new(me.vel.x, me.vel.y, if s.falls then FALL else 0)
    me:set_lifecycle("update")
    tick(me)
end

flying_shot.kind = define.kind {
    id = "flying-shot",
    pool = "attack",
    state = {
        variant = "record:flying-shot-variant",
        wait = "u8",
        falls = "bool",
        range = "u8",
        obstacle = "u8",
    },
    -- `sub_80C60A8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- `sub_80C6248`: a flying shot of `variant` from `owner` at `pos`, with the
-- damage word `damage`, fired as `options` say. It takes the owner's side
-- and flip, and runs later this tick, right after its spawner.
function flying_shot.spawn(owner: Object, pos: Vec3, variant: Variant, damage: number, options: Options?): Object?
    local o = battle.spawn(flying_shot.kind, pos)
    if o then
        local opt: Options = options or {}
        local s = o.state :: State
        s.variant = variant
        s.wait, s.falls, s.range = opt.wait or 0, opt.falls or false, opt.palette or 0
        s.obstacle = opt.obstacle or 0
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return flying_shot
