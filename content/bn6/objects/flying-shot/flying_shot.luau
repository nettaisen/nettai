--!strict
-- The flying shot (the original's attack object #0xB, `sub_80C60A8`):
-- TrnArrw's arrows, the Beast forms' slash waves, a thrown obstacle. It
-- flies forward at a steady speed (and falls, if it was launched to) until
-- it has passed its range in panel centers, and ends on the first thing it
-- hits or when it leaves the field. What differs between its uses is its
-- variant (`flying_shot.variant { ... }`, the original's `byte_80C6038`
-- rows): its collision, element, spark, look, speed, range and status.
-- It may wait some ticks before it sets off, and wear a palette.
-- See docs/engine/objects-and-player.md §B8a.

local element = require("../../lib/element")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local flying_shot = {}

export type State = {
    variant: FlyingShotVariant,
    -- Ticks it waits before it sets off (its second parameter).
    wait: number,
    -- It falls as it flies (its third parameter).
    falls: boolean,
    -- Its palette (its fourth parameter), then the panel centers it has
    -- left to pass (the game reuses the parameter).
    palette: number,
    range: number,
    -- A thrown obstacle's look and animation (the game's ExtraVars word,
    -- kind | animation << 4).
    obstacle: AbsorbedLook?,
    obstacle_anim: number,
}

-- How fast it falls when launched to, 16.16 a tick.
local FALL = -0x1_2000
-- The height of a panel spark (`sub_801A100`), 16.16.
local PANEL_SPARK_Z = 0x10_0000
-- Collision results that show a hit spark (`object_spawnCollisionEffect`),
-- unless the first bit is set too.
local SPARKS, NO_SPARK = 0x3F800000, 1

-- A flying shot variant.
function flying_shot.variant(v: FlyingShotVariant): FlyingShotVariant
    return define.record("flying-shot-variant", v)
end

local ARROW_SPRITE = asset.sprite("bow")
local SHOT_SPRITE = asset.sprite("beast-shot")
local BLADE_SPRITE = asset.sprite("sword")
local SHOT = { is = collision.attack, hits = collision.hits_navis }

-- The variants (the original's rows; `by_number` for the callers that
-- still name one by number).
local ROWS: { [number]: FlyingShotVariant } = {
    [0] = flying_shot.variant {
        collision = { is = collision.thrown_piercing_break, hits = collision.hits_navis },
        hit_mod = 3,
        element = "null",
        spark = sparks.plain,
        sprite = BLADE_SPRITE,
        speed = 0x5_0000,
        range = 8,
    },
    [1] = flying_shot.variant {
        collision = SHOT,
        hit_mod = 3,
        element = "null",
        spark = sparks.heavy,
        sprite = BLADE_SPRITE,
        speed = 0xA_0000,
        range = 8,
    },
    -- TrnArrw's arrow.
    [2] = flying_shot.variant {
        collision = SHOT,
        hit_mod = 1,
        element = "aqua",
        spark = sparks.of_element(2),
        sprite = ARROW_SPRITE,
        anim = 3,
        speed = 0x6_0000,
        range = 8,
        panel_spark = true,
        launch_sound = asset.sound("twang"),
    },
    [3] = flying_shot.variant {
        collision = SHOT,
        hit_mod = 0,
        element = "null",
        spark = sparks.charged,
        sprite = SHOT_SPRITE,
        speed = 0xC_0000,
        range = 8,
    },
    [4] = flying_shot.variant {
        collision = SHOT,
        hit_mod = 1,
        element = "null",
        spark = sparks.charged,
        sprite = SHOT_SPRITE,
        speed = 0x8_0000,
        range = 8,
    },
    [5] = flying_shot.variant {
        collision = SHOT,
        hit_mod = 0,
        element = "null",
        spark = sparks.charged,
        sprite = SHOT_SPRITE,
        speed = 0xC_0000,
        range = 2,
        end_effect = define.effect { sprite = asset.sprite("copy-mark"), anim = 0, palette = 0 },
    },
    -- A thrown obstacle.
    [6] = flying_shot.variant {
        collision = { is = collision.breaking, hits = collision.hits_navis },
        hit_mod = 3,
        element = "null",
        secondary = { "break" },
        spark = sparks.breaking,
        sprite = SHOT_SPRITE,
        speed = 0xA_0000,
        range = 8,
        highlight = true,
        obstacle = true,
    },
}
-- (The Falzar Beast forms' buster fires rows 3 and 5: the shot ahead, and
-- the volley's other two, which fly two panels and burst.)
flying_shot.variants = { arrow = ROWS[2], thrown = ROWS[6], beast_shot = ROWS[3], beast_volley_shot = ROWS[5] }

-- The variant of row `n` (past the table the game reads on into the
-- routine's code: an error).
function flying_shot.by_number(n: number): FlyingShotVariant
    local r = ROWS[n]
    if not r then
        error(string.format("flying shot row %#x reads past byte_80C6038", n))
    end
    return r
end

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
local function panel_spark(me: Object, v: FlyingShotVariant)
    local c = me.collision
    local f = c.hit_flags
    if not bit32.btest(f, SPARKS) or bit32.btest(f, NO_SPARK) or v.spark == nil then
        return
    end
    local p = me.pos
    local x = int.tdiv(int.asr(p.x, 16) + 0xA0, 0x28)
    local y = int.tdiv(int.asr(p.y, 16) + 0x20, 0x18)
    local cx, cy = field.center(x, y)
    battle.spark(me, battle.jitter(0xF, Vec3.new(cx, cy, PANEL_SPARK_Z)), v.spark :: Spark)
end

-- Fly (and fall); each panel center passed spends its range, and it ends
-- when that runs out (leaving its variant's end effect) or it leaves the
-- field.
local function fly(me: Object, s: State)
    local v = s.variant
    local p, vel = me.pos, me.vel
    local cx = field.center(me.panel_x, me.panel_y)
    local x = int.i32(p.x + vel.x)
    me.pos = Vec3.new(x, p.y, p.z + vel.z)
    if crossed(x, p.x, cx) then
        local left = s.range - 1
        s.range = left
        if left <= 0 then
            local effect = v.end_effect
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
    -- panel (region 1) until it ends (none). Drawn only.
    if v.highlight and c.region ~= nil then
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
    local keeps_facing = false
    if v.obstacle then
        -- The absorbed obstacle's look (objects/absorbed-obstacle's; the
        -- game's by the kind in its word's low nibble), its animation (the
        -- next nibble); a look whose sprite is the original's 0x23 keeps
        -- its parts' own facing (`sub_8002EAC`) and is not flipped.
        local thrown = s.obstacle
        if thrown == nil then
            error("a thrown obstacle's flying shot without the obstacle's look")
        end
        look:load(thrown.sprite)
        anim = bit32.band(s.obstacle_anim, 0xF)
        keeps_facing = thrown.own_facing == true
    else
        look:load(v.sprite)
    end
    look.shadow = if v.shadow then "ground" else "with_sprite"
    me.anim = anim
    me.anim_loaded = 0xFF
    me.visible = true
    if not keeps_facing then
        -- (`sub_8002EAC` otherwise: its parts keep their own facing, and
        -- the sprite itself is not flipped. Drawn only.)
        look:set_flip(bit32.bxor(me.alliance, me.flip))
    end
    look.palette = s.palette
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
    local status = v.status
    if status then
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
        palette = "u8",
        range = "u8",
        obstacle = "record:absorbed-look",
        obstacle_anim = "u8",
    },
    -- `sub_80C60A8`.
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

-- `sub_80C6248`: a flying shot from `owner` at `pos`: of `shot.variant`,
-- waiting `shot.wait` ticks, falling if `shot.falls`, in `shot.palette`,
-- with the damage word and, for a thrown obstacle, the obstacle's look and
-- animation. It takes the owner's side and flip, and runs later this tick,
-- right after its spawner.
function flying_shot.launch(owner: Object, pos: Vec3, shot: FlyingShot): Object?
    local o = battle.spawn(flying_shot.kind, pos)
    if o then
        local s = o.state :: State
        s.variant = shot.variant
        s.wait = shot.wait or 0
        s.falls = shot.falls or false
        s.palette = shot.palette or 0
        s.obstacle = shot.obstacle
        s.obstacle_anim = shot.obstacle_anim or 0
        o:set_damage_word(shot.damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- The same with the original's four parameters (the row, the wait, the
-- fall, the palette), for the callers that still pass them.
function flying_shot.spawn(owner: Object, pos: Vec3, params: { number }, damage: number): Object?
    return flying_shot.launch(owner, pos, {
        variant = flying_shot.by_number(params[1]),
        wait = params[2],
        falls = params[3] ~= 0,
        palette = params[4],
        damage = damage,
    })
end

return flying_shot
