--!strict
-- BN5's flying shot (attack object #0x0B, 0x080C5DCC): BN5's own, BN6's (`sub_80C60A8`, objects/flying-shot)
-- without the wait, the fall, the palette, the thrown obstacle and the end effect. It flies forward at its row's
-- speed until it has passed its range in panel centers, and ends on the first thing it hits or when it leaves the
-- field. Its row (its first parameter; the 16-byte rows at 0x080C5D0C) gives its collision, element, spark, status,
-- look, speed and range, and whether it lights its panel. SideBub's bubbles burst into row 7's (chips/sidebub/sidebub).

local collision = require("../rules/collision")
local sparks = require("../lib/sparks")
local statuses = require("../rules/status")

local flying_shot = {}

type Row = {
    -- Its collision (the row's first three bytes: what it is, what it hits, the hit modifier).
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    -- Its element byte.
    element: number,
    -- Its hit spark (0xFF: none).
    spark: Spark?,
    sprite: SpriteAsset,
    anim: number,
    -- It lights the panels it covers (its second parameter).
    highlight: boolean,
    -- The panel centers it passes before it ends (its third parameter).
    range: number,
    -- It casts a shadow on the ground (else its first part is drawn with the sprite).
    shadow: boolean,
    -- The status its hits carry (`object_setCollisionStatusEffect1`).
    status: Status?,
    -- 16.16 a tick, forward.
    speed: number,
}

export type State = {
    -- Its row (its first parameter).
    row: number,
    -- The panel centers it has left to pass (its third parameter).
    range: number,
    -- It lights its panel (its second parameter).
    highlight: boolean,
}

local SHOT = { is = collision.attack, hits = collision.hits_navis }
local ELEC_SHOT = asset.sprite("thunder-ball")

local function row(r: {
    is: CollisionType?,
    hit_mod: number,
    element: number,
    spark: Spark?,
    sprite: SpriteAsset,
    anim: number?,
    highlight: boolean?,
    status: Status?,
    speed: number,
}): Row
    return {
        is = r.is or SHOT.is,
        hits = SHOT.hits,
        hit_mod = r.hit_mod,
        element = r.element,
        spark = r.spark,
        sprite = r.sprite,
        anim = r.anim or 0,
        highlight = r.highlight or false,
        range = 8,
        shadow = false,
        status = r.status,
        speed = r.speed,
    }
end

local BUBBLE = asset.sprite("side-bubble")

-- The rows at 0x080C5D0C.
local ROWS: { [number]: Row } = {
    [0x0] = row {
        is = collision.thrown_piercing_break,
        hit_mod = 3,
        element = 0,
        spark = sparks.plain,
        sprite = asset.sprite("holy-dream-shot"),
        speed = 0x5_0000,
    },
    -- BugCharg's shots (its charger's sprite).
    [0x1] = row { hit_mod = 3, element = 0, spark = sparks.heavy, sprite = asset.sprite("bug-charger"), speed = 0xA_0000 },
    [0x2] = row { hit_mod = 0, element = 3, sprite = ELEC_SHOT, status = statuses.paralyze_90, speed = 0x3_0000 },
    [0x3] = row { hit_mod = 0, element = 3, sprite = ELEC_SHOT, status = statuses.paralyze_90, speed = 0x3_8000 },
    [0x4] = row { hit_mod = 0, element = 3, sprite = ELEC_SHOT, status = statuses.paralyze_90, speed = 0x4_0000 },
    [0x5] = row { hit_mod = 0, element = 3, sprite = ELEC_SHOT, status = statuses.paralyze_90, speed = 0x4_8000 },
    [0x6] = row {
        hit_mod = 1,
        element = 0,
        sprite = asset.sprite("immobilized"),
        anim = 1,
        status = statuses.immobilize_120,
        speed = 0xA_0000,
    },
    -- SideBub's bubble's burst (and three more like it, faster).
    [0x7] = row { hit_mod = 3, element = 2, sprite = BUBBLE, anim = 2, highlight = true, speed = 0x4_0000 },
    [0x8] = row { hit_mod = 3, element = 2, sprite = BUBBLE, anim = 2, highlight = true, speed = 0x4_0000 },
    [0x9] = row { hit_mod = 3, element = 2, sprite = BUBBLE, anim = 2, highlight = true, speed = 0x5_8000 },
    [0xA] = row { hit_mod = 3, element = 2, sprite = BUBBLE, anim = 2, highlight = true, speed = 0x7_0000 },
    [0xB] = row {
        is = collision.thrown,
        hit_mod = 0,
        element = 3,
        sprite = ELEC_SHOT,
        status = statuses.paralyze_90,
        speed = 0x4_0000,
    },
}

-- The rows by name.
flying_shot.rows = { bubble = 0x7 }

local function row_of(n: number): Row
    local r = ROWS[n]
    if not r then
        error(string.format("BN5's flying shot row %#x reads past 0x080C5D0C's rows", n))
    end
    return r
end

-- `sub_800E6E8` (BN5's 0x0800CB08): `c` lies past `a` and up to `b`, either way.
local function crossed(a: number, b: number, c: number): boolean
    if a <= b then
        return c > a and c <= b
    end
    return c > b and c <= a
end

-- Gone: hidden, its region cleared, destroyed next tick.
local function finish(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080C5E80: the hits against it resolve and spark; it ends on a hit, else flies: each panel center passed spends
-- its range, and it ends when that runs out or it leaves the field. A highlighting row lights its panel.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        finish(me)
    else
        local cx = field.center(me.panel_x, me.panel_y)
        local p = me.pos
        local x = int.i32(p.x + me.vel.x)
        me.pos = Vec3.new(x, p.y, p.z)
        local spent = false
        if crossed(x, p.x, cx) then
            local left = s.range - 1
            s.range = left
            spent = left <= 0
        end
        if spent then
            finish(me)
        else
            me:set_panel_from_coordinates()
            me:update_collision_panels()
            if not field.valid(me.panel_x, me.panel_y) then
                finish(me)
            end
        end
    end
    c:present()
    -- `object_highlightCurrentCollisionPanels` (BN5's 0x0800B388): the panels of its region (none once it ends).
    -- Drawn only.
    if s.highlight then
        me:highlight_collision_panels()
    end
end

-- 0x080C5DF0: its look, element, range, speed and collision from its row, on the panel under it. (The game
-- overwrites its link to its owner with its row's address, so the owner is forgotten.)
local function init(me: Object)
    local s = me.state :: State
    local r = row_of(s.row)
    me.related1 = nil
    local look = me.sprite
    look:load(r.sprite)
    look.shadow = if r.shadow then "ground" else "with_sprite"
    me.anim = r.anim
    me.anim_loaded = 0xFF
    me.visible = true
    look:set_flip(bit32.bxor(me.alliance, me.flip))
    me.element = r.element
    s.range = r.range
    me:set_panel_from_coordinates()
    me.vel = Vec3.new(me.facing * r.speed, me.vel.y, me.vel.z)
    s.highlight = r.highlight
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(r.is, r.hits, r.hit_mod)
    local c = me.collision
    c:set_hit_effect(r.spark)
    c.status_base = r.status
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

flying_shot.kind = define.kind {
    id = "flying-shot",
    pool = "attack",
    state = { row = "u8", range = "u8", highlight = "bool" },
    -- 0x080C5DCC.
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

-- 0x080C5EE6: a flying shot of row `row` from `owner` at `pos`, with the damage word; it takes the owner's side and
-- flip, and runs later this tick, right after its spawner.
function flying_shot.launch(owner: Object, pos: Vec3, row: number, damage: number): Object?
    row_of(row)
    local o = battle.spawn(flying_shot.kind, pos)
    if o then
        (o.state :: State).row = row
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return flying_shot
