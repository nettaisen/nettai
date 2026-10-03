--!strict
-- JustcOne's fist (the original's attack object #0xAE, `sub_80DB304`) and
-- the chip's effect (instant chip effect 19, `sub_80DB4B4`): a Break fist
-- falls from 256 pixels up and behind onto the other side's middle panel
-- (column 5 facing right, 2 facing left), 16 pixels a tick for 16 ticks.
-- It hits what it falls through. On a panel that can take it, it lands: a
-- burst, the panels around it crack, two wide hits of 100 (beside it, then
-- diagonally) that land while dimmed too, and nine puffs of dust. Then it
-- stays, blinking, for 30 ticks (unless its panel is gone).

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")

local strike = {}

local SPRITE = asset.sprite("bn6:justice-one")
local FALL_SOUND, LAND_SOUND = asset.sound("bn6:justice-one"), asset.sound("bn6:hit-bomb-1")
-- Break (secondary element 0x10).
local BREAK = 0x10
-- Its own collision (`dword_80DB558`): a thrown breaking attack that hits
-- navis, hit modifier 3, with the breaking spark.
local HIT_MOD = 3
-- How far behind and above it starts, and how fast it falls (both ways),
-- 16.16.
local START, SPEED = 0x100_0000, 0x10_0000
local FALL_TICKS = 0x10
-- What it can land on (`dword_80DB418`): a panel with a body or object on
-- it, or a solid one.
local LANDS = 0x0F800010
-- The landing hits (`dword_80DB564`, `dword_80DB56C`): beside it, then
-- its diagonals, no spark, attacks that hit navis, 100 damage (with the
-- damage's flag bits) and hit modifier 3.
local WAVES = { regions.beside, regions.diagonals }
local NO_SPARK: Spark? = nil
local WAVE_DAMAGE, WAVE_HIT_MOD = 0x64, 3
-- The dust (effect 0) around its panel (`byte_80DB540`: dx facing
-- forward, dy).
local DUST = { { -1, -1 }, { -1, 0 }, { -1, 1 }, { 0, -1 }, { 0, 0 }, { 0, 1 }, { 1, -1 }, { 1, 0 }, { 1, 1 } }
local STAY_TICKS = 0x1E

-- The actions.
local FALL, STAY = 0, 4

-- `sub_80DB324`.
local function init(me: Object)
    me.visible = true
    -- sub_8011504(0xC62, 0x15050300)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        -- The game goes on without one, writing through a null pointer.
        error("JustcOne's fist without a collision slot (sub_80DB324 doesn't check)")
    end
    me:setup_collision(collision.thrown_break, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local facing = me.facing
    me.vel = Vec3.new(facing * SPEED, me.vel.y, SPEED)
    local p = me.pos
    me.pos = Vec3.new(p.x - facing * START, p.y, START)
    me.timer = FALL_TICKS
    battle.play_sound(FALL_SOUND)
    local c = me.collision
    c:set_hit_effect(sparks.breaking)
    c:set_region(nil)
    me.lifecycle = "update"
end

-- `sub_80DB4CE`: a camera shake (magnitude 3, 45 ticks), the landing's
-- hits (running while dimmed) and dust.
local function land(me: Object)
    battle.shake_camera(3, 0x2D)
    local damage = bit32.band(me.damage, 0xF000) + WAVE_DAMAGE
    for _, region in WAVES do
        local box = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = 0,
            z = 0,
            region = region,
            hit_effect = NO_SPARK,
            target = collision.hits_navis,
            self_type = collision.attack,
            damage = damage,
            stamina = me.stamina,
            hit_mod = WAVE_HIT_MOD,
        })
        -- sub_80C53A6
        if box then
            box.run_while_dimmed = true
        end
    end
    local facing = me.facing
    for _, d in DUST do
        local x, y = field.center(me.panel_x + d[1] * facing, me.panel_y + d[2])
        battle.effect(Vec3.new(x, y, 0), effects.explosion)
    end
end

-- `sub_80DB48A`: crack the 3x3 around (x, y) (region 0x0F).
local function crack_around(x: number, y: number)
    for _, d in regions.square.panels or {} do
        field.crack(x + d[1], y + d[2])
    end
end

-- `sub_80DB3B4`: fall; then land, if the panel can take it.
local function fall(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        local p = me.pos
        me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z - me.vel.z)
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    if bit32.btest(field.flags(me.panel_x, me.panel_y), LANDS) then
        battle.effect(me.pos, effects.burst)
        battle.play_sound(LAND_SOUND)
        crack_around(me.panel_x, me.panel_y)
        land(me)
        me.collision:set_region(regions.single)
    end
    me.action = STAY
    me.phase, me.phase_init = 0, 0
end

-- `sub_80DB41C`: stay 30 ticks, blinking; gone at once where its panel
-- is.
local function stay(me: Object)
    local c = me.collision
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if not field.solid(me.panel_x, me.panel_y) then
        battle.effect(me.pos, effects.puff)
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if me.phase_init == 0 then
        me.timer = STAY_TICKS
        me.phase_init = 4
    end
    me.visible = true
    if bit32.btest(me.timer, 2) then
        me.visible = false
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        c:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DB376`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local a = me.action
    if a == FALL then
        fall(me)
    elseif a == STAY then
        stay(me)
    else
        error(string.format("JustcOne's fist action %#x reads past its table", a))
    end
    me:update_sprite()
    c:present()
end

strike.kind = define.kind {
    id = "bn6:justcone/strike",
    pool = "attack",
    -- `sub_80DB304`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80DB474` and `sub_801155A`: the fist of `owner`'s on (x, y), with
-- the damage word `damage`. (It spawns at the registers' values, which
-- its init replaces.)
function strike.spawn(owner: Object, x: number, y: number, element: number, z: number, damage: number): Object?
    local o = battle.spawn(strike.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80DB4B4`, instant chip effect 19: the other side's middle panel,
-- Break.
function strike.instant(user: Object, spec: InstantChipSpec)
    local x = 5 - bit32.bxor(user.alliance, user.flip) * 3
    strike.spawn(user, x, 2, BREAK, spec.z, spec.damage)
end

return strike
