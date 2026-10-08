--!strict
-- SparkMan's spark (attack object #0x22, 0x080D10D0: EXE4's own; EXE6's #0x22 is SpoutMan's ball), which zigzags over
-- the field from the panel ahead of him: each spark is SparkMan's sprite in its animation 8, an attack at navis with
-- the elec spark, its region its panel for 12 ticks; 5 ticks on (10 when it moves on to another column) it sets the
-- next spark on the next panel up its column (down, once its last went down), or, at the column's end, on the next
-- column's panel beside it, turning; 19 ticks on it goes out of sight, and it goes once both are done. A hit on a body
-- or object ends its region and moves it out of sight. Each spark counts itself in SparkMan's `sparks` while it lasts;
-- the chain ends where the next panel is off the field.
--
-- Not reached: its first parameter but 4 (SparkMan's: the delay table 0x080D11AC's row, and whether it counts itself),
-- the other objects' that spawn it.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local spark = {}

type State = {
    -- Its way along its column (Param2: up), and the next spark's panel (Param3, Param4).
    up: boolean,
    next_x: number,
    next_y: number,
    -- SparkMan, whose `sparks` it counts itself in (its +0x4C pointer to his +0x60).
    holder: Object?,
}

local SPRITE = asset.sprite("sparkman")
local SOUND = asset.sound("spark")
-- The ticks to the next spark (0x080D11AC's row 4), twice that when it moves on to another column.
local DELAY = 5
-- It lasts 19 ticks, its region its panel until 7 are left.
local LASTS, REGION_UNTIL = 19, 7
-- What a hit on it must have touched to end its region (a body or an object).
local STOPS = 0x3F80_0000

local function count(me: Object, by: number)
    local h = (me.state :: State).holder
    if h then
        local s = h.state :: any
        s.sparks = int.u32(s.sparks + by)
    end
end

-- Out of sight: its Y's whole part 240.
local function hide(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x, bit32.bor(bit32.lshift(240, 16), bit32.band(p.y, 0xFFFF)), p.z)
end

-- 0x080D1286: the next spark's panel: up or down its column, or at its end the next column's panel beside it, its way
-- turned.
local function aim(me: Object)
    local s = me.state :: State
    local x, y = me.panel_x, me.panel_y
    if s.up and y ~= 1 then
        y -= 1
    elseif not s.up and y ~= 3 then
        y += 1
    else
        x += me.facing
        s.up = not s.up
    end
    s.next_x, s.next_y = x, y
end

-- 0x080D11FA: its region off with 7 ticks left; the next spark when its delay runs out (shown and running while paused
-- and dimmed as it is: the original copies its flags byte); out of sight when its ticks run out; gone once both have.
local function tick(me: Object)
    local s = me.state :: State
    if me.timer == REGION_UNTIL then
        me.collision:set_region(nil)
    end
    if me.timer2 ~= 0 then
        me.timer2 -= 1
        if me.timer2 == 0 then
            local o = spark.spawn(me, s.next_x, s.next_y, me.element, s.up, me.pos.z, s.holder)
            if o then
                o.visible = me.visible
                o.run_while_paused = me.run_while_paused
                o.run_while_dimmed = me.run_while_dimmed
            end
        end
    end
    if me.timer ~= 0 then
        me.timer -= 1
        if me.timer == 0 then
            hide(me)
        end
    end
    if me.timer == 0 and me.timer2 == 0 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- 0x080D11B4: its hits resolve and spark; the battle over, it goes; a hit on a body or object ends its region and
-- moves it out of sight; then its action.
local function update(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if bit32.btest(c.hit_flags, STOPS) then
        c:set_region(nil)
        hide(me)
    end
    if me.action ~= 0 then
        error(string.format("SparkMan's spark's action %#x reads past its table (0x080D11EC)", me.action))
    end
    tick(me)
    c:present()
end

-- 0x080D110A: on a valid panel (else gone at once), counted in SparkMan's sparks; SparkMan's sprite in animation 8,
-- no shadow, facing its side's way, on its panel; an attack at navis (modifier 3, the elec spark), its region its
-- panel; the next spark's panel; its ticks; its sound. (Without a collision record it is freed still counted, as the
-- original leaves it.)
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    count(me, 1)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 8, 8
    sprite:set_animation(8)
    sprite.palette = 0
    sprite:set_flip(me.alliance)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, 3)
    me.collision:set_region(regions.single)
    me.collision:set_hit_effect(sparks.elec)
    me.collision:present()
    aim(me)
    me.timer = LASTS
    local s = me.state :: State
    me.timer2 = if me.panel_x ~= s.next_x then DELAY * 2 else DELAY
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

spark.kind = new.kind {
    id = "sparkman/spark",
    pool = "attack",
    state = { up = "bool", next_x = "u8", next_y = "u8", holder = "object" },
    -- 0x080D10D0: by its state, then its sprite, while dimmed too.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            update(me)
        else
            -- 0x080D10F4: it leaves SparkMan's count; its collision, then itself, freed.
            count(me, -1)
            me.collision:free()
            me:free()
            return
        end
        if me.active then
            me:update_sprite_while_dimmed()
        end
    end,
}

-- 0x080D1252: a spark of `owner`'s side on (x, y) with `element`, going up its column (`up`), at height `z`, with
-- `owner`'s damage word, counted in `holder`'s sparks. (Its position is the spawner's registers: the row, the element
-- and the height, of which its init keeps the height.)
function spark.spawn(owner: Object, x: number, y: number, element: number, up: boolean, z: number, holder: Object?): Object?
    local o = battle.spawn(spark.kind, Vec3.new(y, element, z))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(bit32.bor(owner.damage, bit32.lshift(owner.stamina, 16)))
    o.alliance = owner.alliance
    local s = o.state :: State
    s.up = up
    s.holder = holder
    return o
end

return spark
